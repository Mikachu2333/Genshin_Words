use std::{
    collections::{BTreeMap, HashMap},
    io,
};

use pinyin::ToPinyin;
use serde::Deserialize;

use crate::{
    COMMON_READINGS_PER_CHARACTER, MAX_CHARACTERS_PER_WORD, MIN_READING_FREQUENCY_PERCENT,
    invalid_data,
};

const WEIGHT_DATA: &str = include_str!("../data/pinyin_reading_weights.tsv");

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Rules {
    whitelist: Whitelist,
    blacklist: Blacklist,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Whitelist {
    characters: BTreeMap<char, CharacterReadings>,
    words: BTreeMap<String, BTreeMap<char, String>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CharacterReadings {
    Single(String),
    Pair([String; 2]),
}

impl CharacterReadings {
    fn as_slice(&self) -> &[String] {
        match self {
            Self::Single(reading) => std::slice::from_ref(reading),
            Self::Pair(readings) if readings[0] == readings[1] => &readings[..1],
            Self::Pair(readings) => readings,
        }
    }

    fn validate(&mut self, location: &str) -> io::Result<()> {
        if let Self::Pair(readings) = self {
            readings.sort_unstable();
        }
        for reading in self.as_slice() {
            validate_reading(reading, location)?;
        }
        Ok(())
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Blacklist {
    characters: BTreeMap<char, String>,
}

impl Blacklist {
    fn allows(&self, character: char, reading: &str) -> bool {
        self.characters
            .get(&character)
            .is_none_or(|denied| denied != reading)
    }
}

#[derive(Clone, Copy)]
struct WeightedReading {
    pinyin: &'static str,
    weight: u32,
}

struct WordRule {
    word: String,
    characters: Vec<char>,
    readings: BTreeMap<char, String>,
}

#[derive(Clone, Copy)]
struct WordOverride<'a> {
    pattern: &'a str,
    specificity: usize,
    readings: &'a [String],
}

pub(crate) struct ReadingPolicy {
    character_whitelist: BTreeMap<char, CharacterReadings>,
    word_whitelist: Vec<WordRule>,
    blacklist: Blacklist,
    weights: HashMap<char, Vec<WeightedReading>>,
    cache: HashMap<char, Option<Vec<&'static str>>>,
    minimum_percent: f64,
}

impl ReadingPolicy {
    pub(crate) fn new(rules: Rules) -> io::Result<Self> {
        Self::with_threshold(rules, MIN_READING_FREQUENCY_PERCENT)
    }

    pub(crate) fn from_json(input: &str) -> io::Result<Self> {
        let rules = serde_json::from_str(input)
            .map_err(|error| invalid_data(format!("invalid pronunciation rules: {error}")))?;
        Self::new(rules)
    }

    fn with_threshold(mut rules: Rules, minimum_percent: f64) -> io::Result<Self> {
        if !(0.0..=100.0).contains(&minimum_percent) {
            return Err(invalid_data(
                "reading frequency percentage must be between 0 and 100",
            ));
        }
        validate_rules(&mut rules)?;
        let mut word_whitelist: Vec<_> = rules
            .whitelist
            .words
            .into_iter()
            .map(|(word, readings)| WordRule {
                characters: word.chars().collect(),
                word,
                readings,
            })
            .collect();
        word_whitelist.sort_unstable_by(|a, b| {
            b.characters
                .len()
                .cmp(&a.characters.len())
                .then_with(|| a.word.cmp(&b.word))
        });
        Ok(Self {
            character_whitelist: rules.whitelist.characters,
            word_whitelist,
            blacklist: rules.blacklist,
            weights: parse_weights()?,
            cache: HashMap::new(),
            minimum_percent,
        })
    }

    pub(crate) fn readings_for_word(
        &mut self,
        word: &str,
        line_number: usize,
    ) -> io::Result<Vec<Vec<&str>>> {
        let characters: Vec<_> = word.chars().collect();
        if characters.len() > MAX_CHARACTERS_PER_WORD {
            return Err(invalid_data(format!(
                "line {line_number}: word exceeds the limit of {MAX_CHARACTERS_PER_WORD} characters"
            )));
        }

        // Only context-independent choices are cached. A word whitelist applies
        // to its matched character positions and must never leak into this cache.
        for &character in &characters {
            self.cache.entry(character).or_insert_with(|| {
                common_readings(
                    character,
                    &self.weights,
                    &self.blacklist,
                    self.minimum_percent,
                )
            });
        }

        let overrides = self.whitelist_overrides(word, &characters, line_number)?;
        let mut result = Vec::with_capacity(characters.len());
        for (index, &character) in characters.iter().enumerate() {
            if let Some(readings) = overrides[index] {
                result.push(readings.iter().map(String::as_str).collect());
                continue;
            }
            let readings = self
                .cache
                .get(&character)
                .and_then(Option::as_ref)
                .ok_or_else(|| {
                    invalid_data(format!(
                        "line {line_number}: unsupported character {character:?} at character {}",
                        index + 1
                    ))
                })?;
            if readings.is_empty() {
                return Err(no_permitted_reading(word, character, index, line_number));
            }
            result.push(readings.clone());
        }
        Ok(result)
    }

    pub(crate) fn readings_for_manual_entry<'a>(
        &'a self,
        word: &str,
        pinyin: &'a str,
        line_number: usize,
    ) -> io::Result<Option<Vec<Vec<&'a str>>>> {
        let characters: Vec<_> = word.chars().collect();
        if characters.len() > MAX_CHARACTERS_PER_WORD {
            return Err(invalid_data(format!(
                "line {line_number}: word exceeds the limit of {MAX_CHARACTERS_PER_WORD} characters"
            )));
        }
        if self.character_whitelist.is_empty()
            && self.word_whitelist.is_empty()
            && self.blacklist.characters.is_empty()
        {
            return Ok(None);
        }
        let overrides = self.whitelist_overrides(word, &characters, line_number)?;
        let affected = overrides.iter().any(Option::is_some)
            || characters
                .iter()
                .any(|character| self.blacklist.characters.contains_key(character));
        if !affected {
            return Ok(None);
        }
        let syllables: Vec<_> = pinyin.split(' ').collect();
        if syllables.len() != characters.len() {
            return Err(invalid_data(format!(
                "line {line_number}: applying pronunciation rules to a manual entry requires one pinyin syllable per character"
            )));
        }
        let mut result = Vec::with_capacity(characters.len());
        for (index, &character) in characters.iter().enumerate() {
            if let Some(readings) = overrides[index] {
                result.push(readings.iter().map(String::as_str).collect());
            } else if self.blacklist.allows(character, syllables[index]) {
                result.push(vec![syllables[index]]);
            } else {
                return Err(no_permitted_reading(word, character, index, line_number));
            }
        }
        Ok(Some(result))
    }

    fn whitelist_overrides(
        &self,
        word: &str,
        characters: &[char],
        line_number: usize,
    ) -> io::Result<Vec<Option<&[String]>>> {
        let mut overrides: Vec<Option<WordOverride<'_>>> = vec![None; characters.len()];
        for rule in &self.word_whitelist {
            if rule.characters.len() > characters.len() {
                continue;
            }
            for (start, window) in characters.windows(rule.characters.len()).enumerate() {
                if window != rule.characters {
                    continue;
                }
                for (offset, character) in window.iter().enumerate() {
                    let Some(reading) = rule.readings.get(character) else {
                        continue;
                    };
                    let readings = std::slice::from_ref(reading);
                    let position = start + offset;
                    if let Some(previous) = overrides[position] {
                        if previous.specificity > rule.characters.len() {
                            continue;
                        }
                        if previous.specificity == rule.characters.len()
                            && previous.readings != readings
                        {
                            return Err(invalid_data(format!(
                                "line {line_number}: conflicting word whitelist rules {:?} and {:?} for character {} in {word:?}",
                                previous.pattern,
                                rule.word,
                                position + 1
                            )));
                        }
                    }
                    overrides[position] = Some(WordOverride {
                        pattern: &rule.word,
                        specificity: rule.characters.len(),
                        readings,
                    });
                }
            }
        }

        Ok(overrides
            .into_iter()
            .zip(characters)
            .map(|(override_reading, character)| {
                override_reading.map(|value| value.readings).or_else(|| {
                    self.character_whitelist
                        .get(character)
                        .map(CharacterReadings::as_slice)
                })
            })
            .collect())
    }
}

fn no_permitted_reading(
    word: &str,
    character: char,
    index: usize,
    line_number: usize,
) -> io::Error {
    invalid_data(format!(
        "line {line_number}: blacklist leaves no permitted reading for {character:?} at character {} in {word:?}",
        index + 1
    ))
}

fn common_readings(
    character: char,
    weights: &HashMap<char, Vec<WeightedReading>>,
    blacklist: &Blacklist,
    minimum_percent: f64,
) -> Option<Vec<&'static str>> {
    if let Some(weighted) = weights.get(&character) {
        let maximum = weighted.first()?.weight;
        return Some(
            weighted
                .iter()
                .filter(|reading| {
                    f64::from(reading.weight) * 100.0 >= f64::from(maximum) * minimum_percent
                })
                .take(COMMON_READINGS_PER_CHARACTER)
                .filter(|reading| blacklist.allows(character, reading.pinyin))
                .map(|reading| reading.pinyin)
                .collect(),
        );
    }
    // Missing weights are not assigned invented frequencies. Keep the database's
    // preferred reading; explicit whitelist rules can supply alternatives.
    character.to_pinyin().map(|pinyin| {
        let reading = pinyin.plain();
        if blacklist.allows(character, reading) {
            vec![reading]
        } else {
            Vec::new()
        }
    })
}

fn parse_weights() -> io::Result<HashMap<char, Vec<WeightedReading>>> {
    let mut weights: HashMap<char, Vec<WeightedReading>> = HashMap::new();
    for (index, line) in WEIGHT_DATA.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let character = fields.next().unwrap_or_default();
        let reading = fields.next().unwrap_or_default();
        let weight = fields.next().unwrap_or_default();
        let character = character
            .chars()
            .next()
            .filter(|_| character.chars().count() == 1);
        let weight = weight.parse::<u32>().ok().filter(|weight| *weight > 0);
        let (Some(character), Some(weight)) = (character, weight) else {
            return Err(invalid_data(format!(
                "invalid bundled reading weights on line {}",
                index + 1
            )));
        };
        if !valid_reading(reading) || fields.next().is_some() {
            return Err(invalid_data(format!(
                "invalid bundled reading weights on line {}",
                index + 1
            )));
        }
        let readings = weights.entry(character).or_default();
        if readings.iter().any(|value| value.pinyin == reading) {
            return Err(invalid_data(format!(
                "duplicate bundled reading for {character:?}: {reading}"
            )));
        }
        readings.push(WeightedReading {
            pinyin: reading,
            weight,
        });
    }
    if weights.is_empty() {
        return Err(invalid_data("bundled reading weights are empty"));
    }
    for readings in weights.values_mut() {
        readings
            .sort_unstable_by(|a, b| b.weight.cmp(&a.weight).then_with(|| a.pinyin.cmp(b.pinyin)));
    }
    Ok(weights)
}

fn validate_rules(rules: &mut Rules) -> io::Result<()> {
    for (character, readings) in &mut rules.whitelist.characters {
        readings.validate(&format!("whitelist.characters.{character}"))?;
    }
    for (word, mappings) in &mut rules.whitelist.words {
        let count = word.chars().count();
        if count == 0 || count > MAX_CHARACTERS_PER_WORD || mappings.is_empty() {
            return Err(invalid_data(format!(
                "word whitelist {word:?} must contain 1 to {MAX_CHARACTERS_PER_WORD} characters and at least one reading override"
            )));
        }
        for (character, reading) in mappings {
            if !word.contains(*character) {
                return Err(invalid_data(format!(
                    "word whitelist {word:?} does not contain {character:?}"
                )));
            }
            validate_reading(reading, &format!("whitelist.words.{word}.{character}"))?;
        }
    }
    for (character, reading) in &rules.blacklist.characters {
        validate_reading(reading, &format!("blacklist.characters.{character}"))?;
    }
    Ok(())
}

fn validate_reading(reading: &str, location: &str) -> io::Result<()> {
    if !valid_reading(reading) {
        return Err(invalid_data(format!(
            "{location}: readings must be non-empty lowercase ASCII syllables (use v for ü)"
        )));
    }
    Ok(())
}

fn valid_reading(reading: &str) -> bool {
    !reading.is_empty() && reading.bytes().all(|byte| byte.is_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(json: &str) -> ReadingPolicy {
        ReadingPolicy::from_json(json).unwrap()
    }

    #[test]
    fn uses_frequency_order_and_filters_uncommon_readings() {
        let mut policy = ReadingPolicy::with_threshold(Rules::default(), 0.1).unwrap();
        assert_eq!(
            policy.readings_for_word("什", 1).unwrap(),
            vec![vec!["shen", "shi"]]
        );
        assert_eq!(
            policy.readings_for_word("长", 1).unwrap(),
            vec![vec!["chang", "zhang"]]
        );
        assert_eq!(policy.readings_for_word("哪", 1).unwrap(), vec![vec!["na"]]);
        assert_eq!(
            policy.readings_for_word("还", 1).unwrap(),
            vec![vec!["hai", "huan"]]
        );
        assert_eq!(
            policy.readings_for_word("着", 1).unwrap(),
            vec![vec!["zhe", "zhao"]]
        );
    }

    #[test]
    fn threshold_can_be_adjusted_without_changing_the_two_reading_cap() {
        let mut policy = ReadingPolicy::with_threshold(Rules::default(), 0.0).unwrap();
        assert_eq!(
            policy.readings_for_word("什", 1).unwrap(),
            vec![vec!["shen", "shi"]]
        );
        let readings = policy.readings_for_word("着", 1).unwrap();
        assert_eq!(readings, vec![vec!["zhe", "zhao"]]);
        for invalid in [-1.0, 101.0, f64::NAN, f64::INFINITY] {
            assert!(ReadingPolicy::with_threshold(Rules::default(), invalid).is_err());
        }
    }

    #[test]
    fn character_whitelist_applies_to_every_occurrence() {
        let mut policy = policy(r#"{"whitelist":{"characters":{"莘":["xin","shen"]}}}"#);
        assert_eq!(
            policy.readings_for_word("莘莘", 1).unwrap(),
            vec![vec!["shen", "xin"], vec!["shen", "xin"]]
        );
    }

    #[test]
    fn word_whitelist_applies_only_to_matched_positions_and_does_not_leak_through_cache() {
        let mut policy = policy(r#"{"whitelist":{"words":{"若陀":{"若": "re"}}}}"#);
        assert_eq!(
            policy.readings_for_word("若陀若", 1).unwrap(),
            vec![vec!["re"], vec!["tuo"], vec!["ruo"]]
        );
        assert_eq!(
            policy.readings_for_word("若", 2).unwrap(),
            vec![vec!["ruo"]]
        );
        assert_eq!(
            policy.readings_for_word("若陀", 3).unwrap(),
            vec![vec!["re"], vec!["tuo"]]
        );
    }

    #[test]
    fn multiple_overrides_in_one_word_take_precedence_over_blacklist() {
        let mut policy = policy(
            r#"{
            "whitelist":{"words":{"阿房宫赋":{"阿": "e","房": "pang"}}},
            "blacklist":{"characters":{"阿": "e","房": "pang"}}
        }"#,
        );
        assert_eq!(
            policy.readings_for_word("阿房宫赋阿房", 1).unwrap(),
            vec![
                vec!["e"],
                vec!["pang"],
                vec!["gong"],
                vec!["fu"],
                vec!["a"],
                vec!["fang"]
            ]
        );
    }

    #[test]
    fn blacklist_filters_selected_readings_without_refilling_from_a_third() {
        let rules: Rules =
            serde_json::from_str(r#"{"blacklist":{"characters":{"着": "zhe"}}}"#).unwrap();
        let mut policy = ReadingPolicy::with_threshold(rules, 0.0).unwrap();
        assert_eq!(
            policy.readings_for_word("着", 1).unwrap(),
            vec![vec!["zhao"]]
        );
        let rules: Rules =
            serde_json::from_str(r#"{"blacklist":{"characters":{"着":"zhao"}}}"#).unwrap();
        let mut policy = ReadingPolicy::with_threshold(rules, 0.0).unwrap();
        assert_eq!(
            policy.readings_for_word("着", 1).unwrap(),
            vec![vec!["zhe"]]
        );
    }

    #[test]
    fn character_whitelist_can_override_a_denied_reading() {
        let mut policy = policy(
            r#"{"whitelist":{"characters":{"着": "zhuo"}},"blacklist":{"characters":{"着": "zhuo"}}}"#,
        );
        assert_eq!(
            policy.readings_for_word("着", 1).unwrap(),
            vec![vec!["zhuo"]]
        );
    }

    #[test]
    fn rejects_empty_results_instead_of_restoring_blacklisted_readings() {
        let mut policy = policy(r#"{"blacklist":{"characters":{"甲":"jia"}}}"#);
        assert!(
            policy
                .readings_for_word("甲", 9)
                .unwrap_err()
                .to_string()
                .contains("line 9: blacklist leaves no permitted reading")
        );
    }

    #[test]
    fn longer_word_whitelist_overrides_shorter_and_character_rules() {
        let mut policy = policy(
            r#"{"whitelist":{"characters":{"阿": "a"},"words":{"阿房":{"阿": "a"},"阿房宫赋":{"阿": "e"}}}}"#,
        );
        assert_eq!(policy.readings_for_word("阿房宫赋", 1).unwrap()[0], ["e"]);
    }

    #[test]
    fn overlapping_matches_are_applied() {
        let mut policy = policy(r#"{"whitelist":{"words":{"阿阿":{"阿": "e"}}}}"#);
        assert_eq!(
            policy.readings_for_word("阿阿阿", 1).unwrap(),
            vec![vec!["e"], vec!["e"], vec!["e"]]
        );
    }

    #[test]
    fn equally_specific_conflicting_rules_return_an_error() {
        let mut policy =
            policy(r#"{"whitelist":{"words":{"阿房":{"房": "fang"},"房宫":{"房": "pang"}}}}"#);
        assert!(
            policy
                .readings_for_word("阿房宫", 1)
                .unwrap_err()
                .to_string()
                .contains("conflicting word whitelist rules")
        );
    }

    #[test]
    fn rejects_invalid_configuration() {
        for json in [
            r#"{"whitelist":{"characters":{"阿":["a","e","o"]}}}"#,
            r#"{"whitelist":{"characters":{"阿":[]}}}"#,
            r#"{"whitelist":{"characters":{"阿": "A"}}}"#,
            r#"{"whitelist":{"characters":{"阿": "e pang"}}}"#,
            r#"{"whitelist":{"words":{"阿房":{"若": "re"}}}}"#,
            r#"{"whitelist":{"words":{"":{"阿": "a"}}}}"#,
            r#"{"blacklist":{"character":{"阿": "e"}}}"#,
            r#"{"whitelist":{"characters":{"阿房": "a"}}}"#,
            r#"{"blacklist":{"only":{"阿":"a"}}}"#,
            r#"{"blacklist":{"characters":{"地":["di"]}}}"#,
            r#"{"blacklist":{"characters":{"地":""}}}"#,
            r#"{"blacklist":{"characters":{"地":"DI"}}}"#,
            r#"{"whitelist":{"words":{"包扎":{"扎":["za"]}}}}"#,
            r#"{"whitelist":{"words":{"若陀":{"若":["re","ruo"]}}}}"#,
            r#"{"whitelist":{"words":{"包扎":{"扎":""}}}}"#,
            r#"{"whitelist":{"characters":{"阿":["a"]}}}"#,
        ] {
            assert!(
                ReadingPolicy::from_json(json).is_err(),
                "configuration should fail: {json}"
            );
        }
    }

    #[test]
    fn all_bundled_readings_are_sorted_deduplicated_and_positive() {
        let weights = parse_weights().unwrap();
        assert_eq!(weights.len(), 8183);
        for readings in weights.values() {
            assert!(
                readings
                    .iter()
                    .all(|reading| reading.weight > 0 && valid_reading(reading.pinyin))
            );
            assert!(
                readings
                    .windows(2)
                    .all(|pair| pair[0].weight >= pair[1].weight)
            );
        }
    }

    #[test]
    fn fractional_threshold_includes_its_boundary_and_excludes_lower_frequencies() {
        let weights = HashMap::from([(
            '甲',
            vec![
                WeightedReading {
                    pinyin: "jia",
                    weight: 1000,
                },
                WeightedReading {
                    pinyin: "ga",
                    weight: 1,
                },
            ],
        )]);
        assert_eq!(
            common_readings('甲', &weights, &Blacklist::default(), 0.1).unwrap(),
            ["jia", "ga"]
        );
        assert_eq!(
            common_readings('甲', &weights, &Blacklist::default(), 0.10001).unwrap(),
            ["jia"]
        );
    }

    #[test]
    fn scalar_blacklist_bans_the_specified_reading() {
        let mut policy = policy(r#"{"blacklist":{"characters":{"地":"di"}}}"#);
        assert_eq!(policy.readings_for_word("地", 1).unwrap(), vec![vec!["de"]]);
    }

    #[test]
    fn scalar_character_whitelist_sets_one_reading_and_pairs_keep_two() {
        let mut single = policy(r#"{"whitelist":{"characters":{"莘":"xin"}}}"#);
        assert_eq!(
            single.readings_for_word("莘", 1).unwrap(),
            vec![vec!["xin"]]
        );
        let mut pair = policy(r#"{"whitelist":{"characters":{"莘":["xin","shen"]}}}"#);
        assert_eq!(
            pair.readings_for_word("莘", 1).unwrap(),
            vec![vec!["shen", "xin"]]
        );
        let mut duplicate = policy(r#"{"whitelist":{"characters":{"莘":["xin","xin"]}}}"#);
        assert_eq!(
            duplicate.readings_for_word("莘", 1).unwrap(),
            vec![vec!["xin"]]
        );
    }

    #[test]
    fn scalar_word_whitelist_overrides_blacklist_only_within_the_matched_word() {
        let mut policy = policy(
            r#"{
            "blacklist":{"characters":{"扎":"za"}},
            "whitelist":{"words":{"包扎":{"扎":"za"}}}
        }"#,
        );
        assert_eq!(
            policy.readings_for_word("包扎扎", 1).unwrap(),
            vec![vec!["bao"], vec!["za"], vec!["zha"]]
        );
        assert_eq!(
            policy.readings_for_word("扎", 2).unwrap(),
            vec![vec!["zha"]]
        );
    }
}
