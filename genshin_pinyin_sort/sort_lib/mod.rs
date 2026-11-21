use pinyin::ToPinyin;
use std::collections::HashSet;

const MAX_ENTRY_BYTES: usize = 16 * 1024;

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
    if skip_lines > content.len() {
        return Err(format!(
            "cannot skip {skip_lines} lines from an input containing only {} lines",
            content.len()
        ));
    }

    let mut collection = Vec::with_capacity(content.len().saturating_sub(skip_lines));
    // Exact duplicate lines are dropped, keeping the first occurrence. The key
    // is the whole trimmed line, not the pinyin sort key, so homophones and
    // multiple manual readings of the same word are never collapsed.
    let mut seen: HashSet<&str> = HashSet::new();
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
        if !seen.insert(line) {
            continue;
        }

        let (sort_key, word, reading) = if line.contains('\t') {
            let (key, word) = validate_manual_entry(line, line_number)?;
            let reading = line
                .split(|character| character == '\u{9}')
                .nth(1)
                .unwrap_or_default()
                .to_string();
            (key, word, reading)
        } else {
            let (key, word) = pinyin_sort_key(line, line_number)?;
            (key, word, String::new())
        };
        collection.push((sort_key, word, reading, line.to_string(), index));
    }

    // First pass: the existing pinyin sort, with source order as the final
    // tie-breaker. Homophones and different manual readings all survive.
    collection.sort_by(|(key_a, _, _, _, index_a), (key_b, _, _, _, index_b)| {
        key_a.cmp(key_b).then_with(|| index_a.cmp(index_b))
    });

    // Second pass: group by Chinese word while keeping the pinyin order
    // established by the first pass, both inside and between groups.
    let mut groups: Vec<(String, Vec<(String, String, String, String, usize)>)> = Vec::new();
    for entry in collection {
        let word = entry.1.clone();
        if let Some((_, group)) = groups.iter_mut().find(|(group_word, _)| *group_word == word) {
            group.push(entry);
        } else {
            groups.push((word, vec![entry]));
        }
    }

    for (_, group) in &mut groups {
        group.sort_by(|(key_a, _, reading_a, _, index_a), (key_b, _, reading_b, _, index_b)| {
            reading_a
                .cmp(reading_b)
                .then_with(|| key_a.cmp(key_b))
                .then_with(|| index_a.cmp(index_b))
        });
    }

    Ok(groups
        .into_iter()
        .flat_map(|(_, group)| group)
        .map(|(key, _, _, line, _)| (key, line))
        .collect())
}

fn validate_manual_entry(line: &str, line_number: usize) -> Result<(String, String), String> {
    let mut fields = line.split('\t');
    let word = fields.next().unwrap_or_default();
    let pinyin = fields.next().unwrap_or_default();
    let weight = fields.next().unwrap_or_default();
    if word.is_empty() || pinyin.is_empty() || weight.is_empty() || fields.next().is_some() {
        return Err(format!(
            "line {line_number}: manual entry must contain exactly three non-empty tab-separated fields"
        ));
    }
    if pinyin.split(' ').any(|syllable| {
        syllable.is_empty() || !syllable.bytes().all(|byte| byte.is_ascii_lowercase())
    }) {
        return Err(format!(
            "line {line_number}: Rime pinyin must contain lowercase ASCII syllables separated by single spaces"
        ));
    }
    weight
        .parse::<u64>()
        .map_err(|_| format!("line {line_number}: weight must be a non-negative integer"))?;
    Ok((pinyin.replace(' ', ""), word.to_string()))
}

fn pinyin_sort_key(line: &str, line_number: usize) -> Result<(String, String), String> {
    let mut key = String::new();
    for (character_index, (character, pinyin)) in line.chars().zip(line.to_pinyin()).enumerate() {
        let pinyin = pinyin.ok_or_else(|| {
            format!(
                "line {line_number}: unsupported character {character:?} at character {}",
                character_index + 1
            )
        })?;
        key.push_str(pinyin.with_tone_num_end());
    }
    Ok((key, line.to_string()))
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
        let with_ba = "薄缘的道与光与胤\tba yuan de dao yu guang yu yin\t100";
        let with_bao = "薄缘的道与光与胤\tbao yuan de dao yu guang yu yin\t100";
        let with_bo = "薄缘的道与光与胤\tbo yuan de dao yu guang yu yin\t100";
        let other_word = "薄缘\tbao yuan\t100";

        let sorted = sort_chinese_text(&[with_bo, other_word, with_bao, with_ba], 0).unwrap();
        let lines: Vec<_> = sorted.iter().map(|(_, line)| line.as_str()).collect();

        assert_eq!(lines, vec![other_word, with_ba, with_bao, with_bo]);
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
}
