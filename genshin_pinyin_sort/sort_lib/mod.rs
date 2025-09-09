use pinyin::ToPinyin;
use std::collections::HashMap;

pub fn sort_chinese_text(content: &[&str], skip_lines: usize) -> Vec<(String, String)> {
    let mut collection: HashMap<String, String> = HashMap::new();

    for line in content.iter().skip(skip_lines) {
        if line.is_empty() {
            continue;
        }
        let sentence_py = line
            .to_pinyin()
            .map(|f| match f {
                Some(f) => f.with_tone_num_end(),
                None => "",
            })
            .collect::<Vec<&str>>()
            .join("");
        collection.insert(sentence_py, line.to_string());
    }

    let mut vec_collection = collection.into_iter().collect::<Vec<(String, String)>>();
    vec_collection.sort_by(|(a, _), (b, _)| a.cmp(b));

    vec_collection
}
