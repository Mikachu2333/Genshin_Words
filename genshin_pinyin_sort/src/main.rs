use std::{
    env,
    error::Error,
    ffi::OsString,
    fs::{self, OpenOptions},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use exchange_name_lib::{exchange_rs, resolve_path_rs};
use sort_lib::sort_chinese_text;
use version_lib::VersionInfo;

const HEADER_LINES: usize = 6;
const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 32 * 1024 * 1024;

type AppResult<T> = Result<T, Box<dyn Error>>;

fn main() -> ExitCode {
    match run(env::args_os()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: impl IntoIterator<Item = OsString>) -> AppResult<()> {
    let mut args = args.into_iter();
    let program = args
        .next()
        .unwrap_or_else(|| OsString::from(env!("CARGO_PKG_NAME")));
    let (Some(input_argument), None) = (args.next(), args.next()) else {
        return Err(invalid_input(format!(
            "GenShin Impact Word Sort {}\nUsage: \"{}\" <FullFilePath>",
            env!("CARGO_PKG_VERSION"),
            Path::new(&program).display()
        ))
        .into());
    };

    let current_exe = env::current_exe()?;
    let current_dir = current_exe
        .parent()
        .ok_or_else(|| invalid_input("executable path has no parent directory"))?;
    let (exists, input_path) = resolve_path_rs(Path::new(&input_argument), current_dir)?;
    if !exists || !input_path.is_file() {
        return Err(invalid_input(format!(
            "input path is not a regular file: {}",
            input_path.display()
        ))
        .into());
    }

    let metadata = input_path.metadata()?;
    if metadata.len() > MAX_INPUT_BYTES {
        return Err(invalid_input(format!(
            "input is larger than the {MAX_INPUT_BYTES}-byte safety limit"
        ))
        .into());
    }

    let output_path = intermediate_path(&input_path)?;
    let output = build_sorted_output(&input_path)?;
    if output.len() > MAX_OUTPUT_BYTES {
        return Err(invalid_data(format!(
            "output is larger than the {MAX_OUTPUT_BYTES}-byte safety limit"
        ))
        .into());
    }

    write_new_file(&output_path, &output)?;
    if let Err(error) = exchange_rs(&input_path, &output_path, true) {
        // The exchange API may have encountered a rollback failure. Keep both paths
        // untouched so a user can inspect and recover them safely.
        return Err(format!(
            "failed to replace {} with {}: {error}",
            input_path.display(),
            output_path.display()
        )
        .into());
    }

    fs::remove_file(&output_path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "sorted file is installed, but failed to remove old input at {}: {error}",
                output_path.display()
            ),
        )
    })?;

    println!("Sort finished: {}", input_path.display());
    Ok(())
}

fn intermediate_path(input_path: &Path) -> io::Result<PathBuf> {
    let file_stem = input_path
        .file_stem()
        .ok_or_else(|| invalid_input("input path has no file name"))?;
    let mut file_name = OsString::from(file_stem);
    file_name.push("_out");
    if let Some(extension) = input_path.extension() {
        file_name.push(".");
        file_name.push(extension);
    }
    Ok(input_path.with_file_name(file_name))
}

fn build_sorted_output(input_path: &Path) -> AppResult<Vec<u8>> {
    let input = fs::read_to_string(input_path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to read {} as UTF-8: {error}", input_path.display()),
        )
    })?;
    build_sorted_output_from_text(&input).map_err(Into::into)
}

fn build_sorted_output_from_text(input: &str) -> io::Result<Vec<u8>> {
    let content: Vec<_> = input.lines().collect();
    validate_fixed_header(&content)?;

    let sorted = sort_chinese_text(&content, HEADER_LINES).map_err(invalid_data)?;
    let version = parse_version_line(content[2])?;
    let timestamp = chrono::Local::now().format("%Y.%m.%d %H:%M");

    let mut output = Vec::with_capacity(input.len());
    writeln!(output, "---")?;
    writeln!(output, "name: yuanshen")?;
    writeln!(output, "version: \"{} | {timestamp}\"", version.to_str())?;
    writeln!(output, "sort: original")?;
    writeln!(output, "use_preset_vocabulary: false")?;
    writeln!(output, "...")?;
    writeln!(output)?;
    for (_, line) in sorted {
        writeln!(output, "{line}")?;
    }
    Ok(output)
}

fn validate_fixed_header(content: &[&str]) -> io::Result<()> {
    if content.len() < HEADER_LINES {
        return Err(invalid_data(format!(
            "dictionary must contain the fixed {HEADER_LINES}-line header"
        )));
    }
    if content[0].trim_start_matches('\u{feff}') != "---" {
        return Err(invalid_data("line 1 must be the YAML separator '---'"));
    }
    if !content[2].trim_start().starts_with("version:") {
        return Err(invalid_data("line 3 must contain the version field"));
    }
    if content[5].trim() != "..." {
        return Err(invalid_data("line 6 must be the YAML separator '...'"));
    }
    Ok(())
}

fn parse_version_line(line: &str) -> io::Result<VersionInfo> {
    let (_, value) = line
        .split_once(':')
        .ok_or_else(|| invalid_data("version field is missing ':'"))?;
    let version = value
        .trim()
        .trim_matches(['\'', '"'])
        .split_once('|')
        .map_or(value.trim(), |(version, _)| version)
        .trim();
    VersionInfo::parse(version).map_err(invalid_data)
}

fn write_new_file(path: &Path, content: &[u8]) -> io::Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "failed to create intermediate output {} (remove a stale file if necessary): {error}",
                    path.display()
                ),
            )
        })?;
    let mut writer = BufWriter::new(file);
    if let Err(error) = writer
        .write_all(content)
        .and_then(|()| writer.flush())
        .and_then(|()| writer.get_ref().sync_all())
    {
        drop(writer);
        let _ = fs::remove_file(path);
        return Err(error);
    }
    Ok(())
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "---\nname: yuanshen\nversion: \"6.8(5) | old\"\nsort: original\nuse_preset_vocabulary: false\n...\n\n";

    #[test]
    fn fixed_header_is_validated_without_general_yaml_parsing() {
        let error = build_sorted_output_from_text("---\nname: x\n").unwrap_err();
        assert!(error.to_string().contains("fixed 6-line header"));

        let error = build_sorted_output_from_text(
            "---\nname: x\nnot-version: x\nsort: original\nuse_preset_vocabulary: false\n...\n",
        )
        .unwrap_err();
        assert!(error.to_string().contains("line 3"));
    }

    #[test]
    fn increments_version_and_keeps_homophones() {
        let input = format!("{HEADER}意义\n异议\n");
        let output = String::from_utf8(build_sorted_output_from_text(&input).unwrap()).unwrap();

        assert!(output.contains("version: \"6.8(6) |"));
        assert!(output.contains("意义\n"));
        assert!(output.contains("异议\n"));
    }

    #[test]
    fn duplicate_entries_are_removed_from_final_output() {
        let input = format!("{HEADER}白沙皇\n白沙皇\n白沙皇\n");
        let output = String::from_utf8(build_sorted_output_from_text(&input).unwrap()).unwrap();
        assert_eq!(output.matches("白沙皇").count(), 1);
    }

    #[test]
    fn keeps_multiple_manual_readings() {
        let input = format!(
            "{HEADER}薄缘的道与光与胤\tbao yuan de dao yu guang yu yin\t100\n薄缘的道与光与胤\tbo yuan de dao yu guang yu yin\t100\n"
        );
        let output = String::from_utf8(build_sorted_output_from_text(&input).unwrap()).unwrap();
        assert_eq!(output.matches("薄缘的道与光与胤").count(), 2);
        assert!(output.contains("\tbao yuan de dao yu guang yu yin\t100"));
        assert!(output.contains("\tbo yuan de dao yu guang yu yin\t100"));
    }

    #[test]
    fn output_path_supports_compound_and_non_utf8_safe_components() {
        assert_eq!(
            intermediate_path(Path::new("dictionary.dict.yaml")).unwrap(),
            PathBuf::from("dictionary.dict_out.yaml")
        );
        assert_eq!(
            intermediate_path(Path::new("dictionary")).unwrap(),
            PathBuf::from("dictionary_out")
        );
    }
}
