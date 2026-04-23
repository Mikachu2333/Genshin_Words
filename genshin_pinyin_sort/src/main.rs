use std::{env, io::Write, path::PathBuf};

use name_exchanger_rs::{exchange_rs, resolve_path_rs};
use sort_lib::sort_chinese_text;
use version_lib::VersionInfo;

fn main() {
    let arg = std::env::args().collect::<Vec<String>>();
    if arg.len() != 2 {
        println!("GenShin Impact Word Sort\t");
        println!("Version: {}", env!("CARGO_PKG_VERSION"),);
        println!("Usage:");
        println!(".\"{}\" <FullFilePath>", arg[0]);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        return;
    }

    let (path, out_path) = calc(arg[1].clone());

    let result = exchange_rs(&path, &out_path, true);
    if result.is_err() {
        eprintln!("Error Info: {}", result.err().unwrap());
    } else {
        std::fs::remove_file(out_path).unwrap();
        println!("---------------------\nAll Done.\n");
    }
}

fn calc(file_path: String) -> (PathBuf, PathBuf) {
    let binding = std::env::current_exe().unwrap();
    let current_dir = binding.parent().unwrap();

    let (is_exist, checked_path) =
        resolve_path_rs(PathBuf::from(file_path).as_ref(), current_dir).unwrap_or_default();
    if !is_exist {
        panic!("Not Exist.")
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

    let if_skip: usize = if content[0] == "---" { 6 } else { 0 };

    let vec_collection = sort_chinese_text(&content, if_skip);

    let date = format!(
        "---\nname: yuanshen\nversion: \"{}\"\nsort: origin\nuse_preset_vocabulary: false\n...\n\n",
        parse_date(content[2])
    );
    out_file.write_all(date.as_bytes()).unwrap();

    for (_, i) in vec_collection {
        out_file.write_all(format!("{}\n", i).as_bytes()).unwrap();
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
