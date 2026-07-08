use pinyin::ToPinyinMulti;
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

static DEBUG: bool = cfg!(debug_assertions);

fn main() {
    let args = std::env::args().collect::<Vec<String>>();

    if args.len() != 2 && !DEBUG {
        println!("GenShin Impact Word to Pinyin\t");
        println!("Version: {}", env!("CARGO_PKG_VERSION"),);
        println!("Usage:");
        println!(".\"{}\" <FullFilePath>", args[0]);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        return;
    }

    let path_in = {
        if DEBUG {
            PathBuf::from(
                r"D:\Temp\Genshin_Words\genshin_pinyin_add\target\debug\yuanshen.dict.yaml",
            )
        } else {
            PathBuf::from(&args[1])
        }
    };
    if !path_in.is_file() {
        panic!("Input path is not a valid file!");
    }
    let path_out = {
        let mut temp = path_in.clone();
        temp.set_file_name("yuanshen_fcitx5.txt");
        temp
    };

    println!("Input path: {}", path_in.display());
    println!("Output path: {}", path_out.display());

    let content = delete_useless(&path_in);

    let mut file_out = fs::File::create(&path_out).unwrap();

    for line in content {
        let checked_line = line.trim();
        if checked_line.is_empty() {
            continue;
        }

        // 如果该行已包含\t分隔的拼音，直接使用，不自动生成
        if checked_line.contains("\t") {
            writeln!(file_out, "{}", checked_line).unwrap();
            continue;
        }

        let mut char_pinyins_list: Vec<Vec<&str>> = Vec::new();
        for multi in checked_line.to_pinyin_multi().flatten() {
            let mut pinyins: Vec<&str> = Vec::with_capacity(2);
            for i in 0..2 {
                if let Some(p) = multi.get_opt(i) {
                    pinyins.push(p.plain());
                }
            }
            pinyins.sort();
            pinyins.dedup();
            char_pinyins_list.push(pinyins);
        }

        if char_pinyins_list.is_empty() {
            continue;
        }

        let mut combinations: Vec<String> = vec![String::new()];
        for pinyins in char_pinyins_list {
            let mut next_combinations = Vec::new();
            for prev in combinations {
                for p in &pinyins {
                    let mut new_str = prev.clone();
                    if !new_str.is_empty() {
                        new_str.push('\'');
                    }
                    new_str.push_str(p);
                    next_combinations.push(new_str);
                }
            }
            combinations = next_combinations;
        }

        for pinyin_str in combinations {
            writeln!(file_out, "{}\t{}\t0", checked_line, pinyin_str).unwrap();
        }
    }

    file_out.flush().unwrap();
    println!("Done! Output path: {}", path_out.display());
}

fn delete_useless(p: &Path) -> Vec<String> {
    let original = fs::read_to_string(p).unwrap();
    let mut result: Vec<String> = Vec::new();

    let mut judge = 0;
    for line in original.lines() {
        let temp = line.trim();
        if temp == "---" || temp == "..." {
            judge += 1;
            continue;
        }

        if judge < 2 {
            continue;
        } else if judge == 2 {
            result.push(line.to_string());
        } else {
            panic!("Error yuanshen.dict.yaml format!");
        }
    }
    if judge < 2 {
        eprintln!(
            "Warning: no closing YAML separator ('---' or '...') found; no content extracted."
        );
    }
    result
}
