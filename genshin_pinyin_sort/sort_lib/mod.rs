use pinyin::ToPinyin;
use std::collections::{HashMap, HashSet};

const MAX_ENTRY_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy)]
enum PinyinFormat {
    Rime,
    Fcitx5,
}

impl PinyinFormat {
    const fn separator(self) -> char {
        match self {
            Self::Rime => ' ',
            Self::Fcitx5 => '\'',
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Rime => "Rime",
            Self::Fcitx5 => "Fcitx5",
        }
    }
}

struct SortEntry<'a> {
    key: String,
    word: &'a str,
    reading: &'a str,
    line: &'a str,
    source_index: usize,
    group_index: usize,
}

/// A validated, borrowed three-field Rime dictionary entry.
#[derive(Debug)]
pub struct ManualEntry<'a> {
    pub word: &'a str,
    pub pinyin: &'a str,
    pub weight: &'a str,
}

/// Sorts dictionary content by pinyin while preserving homophones.
///
/// # Errors
///
/// Returns an error for an invalid skip count, malformed manual entry,
/// oversized entry, or character unsupported by the pinyin database.
pub fn sort_chinese_text(
    content: &[&str],
    skip_lines: usize,
) -> Result<Vec<(String, String)>, String> {
    let mut entries = collect_entries(content, skip_lines, PinyinFormat::Rime, true)?;
    sort_entries(&mut entries);
    Ok(entries
        .into_iter()
        .map(|entry| (entry.key, entry.line.to_owned()))
        .collect())
}

/// Sorts generated Fcitx5 rows using the same word grouping and pinyin order
/// as [`sort_chinese_text`]. Equal rows and homophones retain their source order.
///
/// # Errors
///
/// Returns an error for an oversized row or malformed three-field Fcitx5 entry.
pub fn sort_fcitx5_lines<'a>(content: &[&'a str]) -> Result<Vec<&'a str>, String> {
    let mut entries = collect_entries(content, 0, PinyinFormat::Fcitx5, false)?;
    sort_entries(&mut entries);
    Ok(entries.into_iter().map(|entry| entry.line).collect())
}

fn collect_entries<'a>(
    content: &[&'a str],
    skip_lines: usize,
    format: PinyinFormat,
    deduplicate: bool,
) -> Result<Vec<SortEntry<'a>>, String> {
    if skip_lines > content.len() {
        return Err(format!(
            "cannot skip {skip_lines} lines from an input containing only {} lines",
            content.len()
        ));
    }

    let capacity = content.len().saturating_sub(skip_lines);
    let mut collection = Vec::with_capacity(capacity);
    // Exact duplicate lines are dropped, keeping the first occurrence. The key
    // is the whole trimmed line, not the pinyin sort key, so homophones and
    // multiple manual readings of the same word are never collapsed.
    let mut seen: HashSet<&str> = HashSet::with_capacity(if deduplicate { capacity } else { 0 });
    for (index, line) in content.iter().enumerate().skip(skip_lines) {
        let line_number = index + 1;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.len() > MAX_ENTRY_BYTES {
            return Err(format!(
                "line {line_number}: entry exceeds the {MAX_ENTRY_BYTES}-byte safety limit"
            ));
        }
        if deduplicate && !seen.insert(line) {
            continue;
        }

        let (key, word, reading) = if line.contains('\t') || matches!(format, PinyinFormat::Fcitx5)
        {
            let manual = parse_entry(line, line_number, format)?;
            (
                manual.pinyin.replace(format.separator(), ""),
                manual.word,
                manual.pinyin,
            )
        } else {
            (pinyin_sort_key(line, line_number)?, line, "")
        };
        collection.push(SortEntry {
            key,
            word,
            reading,
            line,
            source_index: index,
            group_index: 0,
        });
    }
    Ok(collection)
}

fn sort_entries(entries: &mut [SortEntry<'_>]) {
    // A word's earliest pinyin key determines its group's position. Explicit
    // source indices keep equal keys deterministic even with unstable sorting.
    entries.sort_unstable_by(|a, b| {
        a.key
            .cmp(&b.key)
            .then_with(|| a.source_index.cmp(&b.source_index))
    });

    let mut groups = HashMap::with_capacity(entries.len());
    for entry in entries.iter_mut() {
        let next_group = groups.len();
        entry.group_index = *groups.entry(entry.word).or_insert(next_group);
    }

    entries.sort_unstable_by(|a, b| {
        a.group_index
            .cmp(&b.group_index)
            .then_with(|| a.reading.cmp(b.reading))
            .then_with(|| a.key.cmp(&b.key))
            .then_with(|| a.source_index.cmp(&b.source_index))
    });
}

/// Validates a manual Rime row while borrowing its original fields.
///
/// # Errors
///
/// Returns an error for missing/extra fields, invalid pinyin, or an invalid weight.
pub fn parse_manual_entry(line: &str, line_number: usize) -> Result<ManualEntry<'_>, String> {
    parse_entry(line, line_number, PinyinFormat::Rime)
}

fn parse_entry(
    line: &str,
    line_number: usize,
    format: PinyinFormat,
) -> Result<ManualEntry<'_>, String> {
    let mut fields = line.split('\t');
    let word = fields.next().unwrap_or_default();
    let pinyin = fields.next().unwrap_or_default();
    let weight = fields.next().unwrap_or_default();
    if word.is_empty() || pinyin.is_empty() || weight.is_empty() || fields.next().is_some() {
        return Err(format!(
            "line {line_number}: manual entry must contain exactly three non-empty tab-separated fields"
        ));
    }
    if pinyin.split(format.separator()).any(|syllable| {
        syllable.is_empty() || !syllable.bytes().all(|byte| byte.is_ascii_lowercase())
    }) {
        return Err(format!(
            "line {line_number}: {} pinyin must contain lowercase ASCII syllables separated by single {}",
            format.name(),
            if matches!(format, PinyinFormat::Rime) {
                "spaces"
            } else {
                "apostrophes"
            },
        ));
    }
    weight
        .parse::<u64>()
        .map_err(|_| format!("line {line_number}: weight must be a non-negative integer"))?;
    Ok(ManualEntry {
        word,
        pinyin,
        weight,
    })
}

fn pinyin_sort_key(line: &str, line_number: usize) -> Result<String, String> {
    let mut key = String::with_capacity(line.len());
    for (character_index, (character, pinyin)) in line.chars().zip(line.to_pinyin()).enumerate() {
        let pinyin = pinyin.ok_or_else(|| {
            format!(
                "line {line_number}: unsupported character {character:?} at character {}",
                character_index + 1
            )
        })?;
        key.push_str(pinyin.plain());
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_bare_words_are_deduplicated() {
        let sorted = sort_chinese_text(&["白沙皇", "白沙皇", "白沙皇"], 0).unwrap();
        assert_eq!(sorted.len(), 1);
        assert_eq!(sorted[0].1, "白沙皇");
    }

    #[test]
    fn duplicate_identical_manual_entries_are_deduplicated() {
        let duplicate = "薄缘的道与光与胤\tbao yuan de dao yu guang yu yin\t100";
        let sorted = sort_chinese_text(&[duplicate, duplicate], 0).unwrap();
        assert_eq!(sorted.len(), 1);
        assert_eq!(sorted[0].1, duplicate);
    }

    #[test]
    fn homophones_are_not_overwritten() {
        let sorted = sort_chinese_text(&["意义", "异议"], 0).unwrap();
        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0].1, "意义");
        assert_eq!(sorted[1].1, "异议");
    }

    #[test]
    fn alternate_manual_readings_stay_with_the_same_word() {
        let shortened = "薄缘的道与光与胤\tba yuan de dao yu guang yu yin\t100";
        let primary = "薄缘的道与光与胤\tbao yuan de dao yu guang yu yin\t100";
        let alternate = "薄缘的道与光与胤\tbo yuan de dao yu guang yu yin\t100";
        let other_word = "薄缘\tbao yuan\t100";

        let sorted = sort_chinese_text(&[alternate, other_word, primary, shortened], 0).unwrap();
        let lines: Vec<_> = sorted.iter().map(|(_, line)| line.as_str()).collect();

        assert_eq!(lines, vec![other_word, shortened, primary, alternate]);
    }

    #[test]
    fn manual_entries_are_sorted_by_pinyin_and_preserved() {
        let sorted = sort_chinese_text(
            &[
                "薄缘的道与光与胤\tbo yuan de dao yu guang yu yin\t100",
                "薄缘的道与光与胤\tbao yuan de dao yu guang yu yin\t100",
            ],
            0,
        )
        .unwrap();
        assert!(sorted[0].1.contains("\tbao yuan "));
        assert!(sorted[1].1.contains("\tbo yuan "));
    }

    #[test]
    fn malformed_manual_entry_returns_an_error() {
        let error = sort_chinese_text(&["词条\tci tiao"], 0).unwrap_err();
        assert!(error.contains("exactly three"));

        let error = sort_chinese_text(&["词条\tci'tiao\t100"], 0).unwrap_err();
        assert!(error.contains("separated by single spaces"));

        let error = sort_chinese_text(&["词条\tci tiao\t-1"], 0).unwrap_err();
        assert!(error.contains("non-negative integer"));
    }

    #[test]
    fn unsupported_character_returns_an_error() {
        let error = sort_chinese_text(&["派蒙2"], 0).unwrap_err();
        assert!(error.contains("unsupported character '2'"));
    }

    #[test]
    fn excessive_skip_is_rejected() {
        assert!(sort_chinese_text(&["词条"], 2).is_err());
    }

    #[test]
    fn automatic_and_manual_keys_use_the_same_plain_pinyin() {
        let manual = "安康\tan kang\t100";
        let sorted = sort_chinese_text(&[manual, "安宁"], 0).unwrap();
        assert_eq!(sorted[0], ("ankang".to_owned(), manual.to_owned()));
        assert_eq!(sorted[1], ("anning".to_owned(), "安宁".to_owned()));
    }

    #[test]
    fn tone_does_not_disrupt_pinyin_alphabetical_order() {
        let sorted = sort_chinese_text(&["阿", "啊"], 0).unwrap();
        assert_eq!(
            sorted
                .iter()
                .map(|entry| entry.0.as_str())
                .collect::<Vec<_>>(),
            ["a", "a"]
        );
        assert_eq!(
            sorted
                .iter()
                .map(|entry| entry.1.as_str())
                .collect::<Vec<_>>(),
            ["阿", "啊"]
        );
    }

    #[test]
    fn rime_and_fcitx5_share_group_order_and_preserve_equal_rows() {
        let rime = [
            "薄缘\tbo yuan\t100",
            "意义\tyi yi\t100",
            "薄缘\tbao yuan\t100",
            "异议\tyi yi\t100",
        ];
        let fcitx: Vec<_> = rime.iter().map(|line| line.replace(' ', "'")).collect();
        let references: Vec<_> = fcitx.iter().map(String::as_str).collect();
        let expected: Vec<_> = sort_chinese_text(&rime, 0)
            .unwrap()
            .into_iter()
            .map(|(_, line)| line.replace(' ', "'"))
            .collect();
        assert_eq!(sort_fcitx5_lines(&references).unwrap(), expected);

        let duplicate = "意义\tyi'yi\t100";
        assert_eq!(
            sort_fcitx5_lines(&[duplicate, duplicate]).unwrap(),
            [duplicate, duplicate]
        );
    }

    #[test]
    fn fcitx5_format_is_validated() {
        assert!(
            sort_fcitx5_lines(&["派蒙\tpai meng\t100"])
                .unwrap_err()
                .contains("apostrophes")
        );
        assert!(sort_fcitx5_lines(&["派蒙\tpai''meng\t100"]).is_err());
        assert!(sort_fcitx5_lines(&["派蒙\tpai'meng\t-1"]).is_err());
    }

    #[test]
    fn entries_with_distinct_weights_survive_in_source_order() {
        let first = "派蒙\tpai meng\t200";
        let second = "派蒙\tpai meng\t100";
        let sorted = sort_chinese_text(&[first, second, first], 0).unwrap();
        assert_eq!(
            sorted
                .iter()
                .map(|entry| entry.1.as_str())
                .collect::<Vec<_>>(),
            [first, second]
        );
    }

    #[test]
    fn oversized_entries_are_rejected() {
        let oversized = "阿".repeat(MAX_ENTRY_BYTES);
        assert!(
            sort_chinese_text(&[&oversized], 0)
                .unwrap_err()
                .contains("safety limit")
        );
    }
}
