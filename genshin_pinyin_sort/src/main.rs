use pinyin::ToPinyin;
use std::{io::Write, path::PathBuf};

use version_lib::VersionInfo;
use exchange_lib::exchange;

fn main() {
    let arg = std::env::args().collect::<Vec<String>>();
    if arg.len() != 2 {
        println!("GenShin Impact Word Sort\t");
        println!("Version: {}", env!("CARGO_PKG_VERSION"),);
        println!("Usage:");
        println!(".\"{}\" <FullFilePath>", arg[0]);
        return;
    }

    let (path, out_path) = calc(arg[0].clone(), arg[1].clone());

    let result = exchange(
        path.to_str().unwrap().to_string(),
        out_path.to_str().unwrap().to_string(),
    );
    if result != 0 {
        eprintln!("Error Code: {}", result);
    } else {
        std::fs::remove_file(out_path).unwrap();
        println!("---------------------\nAll Done.\n");
        return;
    }
}

fn calc(exe_path: String, file_path: String) -> (PathBuf, PathBuf) {
    let checked_path = {
        let temp = PathBuf::from(
            file_path
                .replace("\r\n", "\n")
                .trim_matches(&['"', '\'', '\\', '/', '\t', '\n', '\r', '`']),
        );

        let unchecked_path = if !temp.is_file() || !temp.exists() {
            panic!()
        } else if temp.is_relative() {
            PathBuf::from(exe_path).join(temp)
        } else {
            temp
        };
        unchecked_path.canonicalize().unwrap()
    };

    //dbg!("{}", checked_path.display());

    let ext = checked_path.extension().unwrap().to_str().unwrap();
    let output_path = PathBuf::from(format!(
        "{}_out.{}",
        checked_path
            .to_str()
            .unwrap()
            .trim_end_matches(&format!(".{}", ext)),
        ext
    ));

    let mut out_file = std::fs::File::create(&output_path).unwrap();
    let file = std::fs::read_to_string(&checked_path).unwrap();
    let content = file.lines().collect::<Vec<&str>>();

    let mut collection: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    let if_skip: usize = if content[0] == "---" { 6 } else { 0 };

    for line in content.iter().skip(if_skip) {
        if line.is_empty() {
            continue;
        }
        let sentence_py = line
            .to_pinyin()
            .map(|f| match f {
                Some(f) => f.with_tone_num_end(),
                None => {
                    //println!("{}", line);
                    ""
                }
            })
            .collect::<Vec<&str>>()
            .join("");
        collection.insert(sentence_py, line.to_string());
    }

    let mut vec_collection = collection.into_iter().collect::<Vec<(String, String)>>();
    vec_collection.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));

    let date = format!(
        "---\nname: yuanshen\nversion: \"{}\"\nsort: origin\nuse_preset_vocabulary: false\n...\n\n",
        parse_date(content[2])
    );
    out_file.write(date.as_bytes()).unwrap();

    for (_, i) in vec_collection {
        out_file.write(format!("{}\n", i).as_bytes()).unwrap();
    }
    out_file.flush().unwrap();

    println!("Sort Finished.");

    (checked_path, output_path)
}

fn parse_date(sentence: &str) -> String {
    //version: "5.8(7) | 2025.08.27 14:43"

    let original_data = {
        let temp1 = sentence.splitn(2, ':').collect::<Vec<&str>>();
        let temp2 = temp1[1].replace('"', "").replace("'", "");
        let temp = temp2.split('|').map(String::from).collect::<Vec<String>>();
        temp[0].to_string()
        //5.8(7)
    };

    let parsed = VersionInfo::from_str(&original_data);

    let date = chrono::Local::now().format("%Y.%m.%d %H:%M").to_string();

    format!("{} | {}", parsed.to_str(), date)
}
