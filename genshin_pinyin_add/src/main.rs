mod readings;

use readings::{ReadingPolicy, Rules};
use sort_lib::{parse_manual_entry, sort_fcitx5_lines};
use std::{
    env,
    error::Error,
    ffi::OsString,
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};
use tempfile::NamedTempFile;

const OUTPUT_FILE_NAME: &str = "yuanshen_fcitx5.txt";
const GENERATED_ENTRY_WEIGHT: &str = "100";
const COMMON_READINGS_PER_CHARACTER: usize = 2;
// Keep readings with at least this percentage of the character's highest weight.
// Set 0 to admit all weighted readings before applying the two-reading cap.
const MIN_READING_FREQUENCY_PERCENT: f64 = 0.1;
const RULES_FILE_NAME: &str = "pinyin_rules.json";
const MAX_RULES_BYTES: u64 = 1024 * 1024;
const MAX_CHARACTERS_PER_WORD: usize = 256;
const MAX_COMBINATIONS_PER_WORD: usize = 1_000_000;
const MAX_ENTRY_BYTES: usize = 16 * 1024;
const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 512 * 1024 * 1024;

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
    let (path_in, rules_argument) = match (args.next(), args.next(), args.next()) {
        (Some(path), rules, None) => (PathBuf::from(path), rules),
        _ => {
            return Err(invalid_input(format!(
                "GenShin Impact Word to Pinyin {}\nUsage: \"{}\" <FullFilePath> [RulesJsonPath]",
                env!("CARGO_PKG_VERSION"),
                Path::new(&program).display()
            ))
            .into());
        }
    };

    if !path_in.is_file() {
        return Err(invalid_input(format!(
            "input path is not a regular file: {}",
            path_in.display()
        ))
        .into());
    }

    let path_out = path_in.with_file_name(OUTPUT_FILE_NAME);
    if path_in == path_out {
        return Err(invalid_input("input and output paths must be different").into());
    }
    let output_dir = path_out
        .parent()
        .ok_or_else(|| invalid_input("output path has no parent directory"))?;

    println!("Input path: {}", path_in.display());
    println!("Output path: {}", path_out.display());

    let input_metadata = path_in.metadata().map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to inspect input {}: {error}", path_in.display()),
        )
    })?;
    if input_metadata.len() > MAX_INPUT_BYTES {
        return Err(invalid_input(format!(
            "input is larger than the {MAX_INPUT_BYTES}-byte safety limit"
        ))
        .into());
    }

    let mut policy = load_reading_policy(&path_in, rules_argument.as_deref().map(Path::new))?;

    let input = File::open(&path_in).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to open input {}: {error}", path_in.display()),
        )
    })?;
    let mut temporary = NamedTempFile::new_in(output_dir).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "failed to create a temporary output in {}: {error}",
                output_dir.display()
            ),
        )
    })?;

    {
        let mut writer = BufWriter::new(temporary.as_file_mut());
        convert(BufReader::new(input), &mut writer, &mut policy)?;
        writer.flush()?;
    }
    temporary.as_file().sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temporary
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o644))?;
    }
    temporary.persist(&path_out).map_err(|error| {
        io::Error::new(
            error.error.kind(),
            format!(
                "failed to atomically replace output {}: {}",
                path_out.display(),
                error.error
            ),
        )
    })?;

    println!("Done! Output path: {}", path_out.display());
    Ok(())
}

fn load_reading_policy(input: &Path, explicit_rules: Option<&Path>) -> io::Result<ReadingPolicy> {
    let default_path = input.with_file_name(RULES_FILE_NAME);
    let path = explicit_rules.unwrap_or(&default_path);
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound && explicit_rules.is_none() => {
            return ReadingPolicy::new(Rules::default());
        }
        Err(error) => {
            return Err(io::Error::new(
                error.kind(),
                format!(
                    "failed to open pronunciation rules {}: {error}",
                    path.display()
                ),
            ));
        }
    };
    let mut json = String::new();
    file.take(MAX_RULES_BYTES + 1).read_to_string(&mut json)?;
    if u64::try_from(json.len()).unwrap_or(u64::MAX) > MAX_RULES_BYTES {
        return Err(invalid_data(format!(
            "pronunciation rules exceed the {MAX_RULES_BYTES}-byte safety limit"
        )));
    }
    ReadingPolicy::from_json(&json)
        .map_err(|error| invalid_data(format!("{}: {error}", path.display())))
}

fn convert(
    reader: impl BufRead,
    writer: &mut impl Write,
    policy: &mut ReadingPolicy,
) -> io::Result<()> {
    // Bound expansion before allocating the shared sorter's keys. The final rows
    // are sorted after derivation, including both generated and manual readings.
    let mut generated = Vec::new();
    let mut generated_writer = LimitedWriter::new(&mut generated, MAX_OUTPUT_BYTES);
    let mut lines = reader.lines().enumerate();
    let (first_number, first_line) = next_line(&mut lines)?
        .ok_or_else(|| invalid_data("input is empty; expected YAML frontmatter"))?;
    validate_line_length(&first_line, first_number)?;
    let first_line = first_line.trim_start_matches('\u{feff}');
    if first_line.trim() != "---" {
        return Err(invalid_data(format!(
            "line {first_number}: expected opening YAML separator '---'"
        )));
    }

    let mut found_closing_separator = false;
    while let Some((line_number, line)) = next_line(&mut lines)? {
        validate_line_length(&line, line_number)?;
        if matches!(line.trim(), "---" | "...") {
            found_closing_separator = true;
            break;
        }
    }
    if !found_closing_separator {
        return Err(invalid_data(
            "no closing YAML separator ('---' or '...') found",
        ));
    }

    for (zero_based_line_number, line) in lines {
        let line = line?;
        let line_number = zero_based_line_number + 1;
        validate_line_length(&line, line_number)?;
        let checked_line = line.trim();
        if checked_line.is_empty() {
            continue;
        }
        if matches!(checked_line, "---" | "...") {
            return Err(invalid_data(format!(
                "line {line_number}: unexpected YAML separator in dictionary content"
            )));
        }

        if checked_line.contains('\t') {
            let manual = parse_manual_entry(checked_line, line_number).map_err(invalid_data)?;
            if let Some(readings) =
                policy.readings_for_manual_entry(manual.word, manual.pinyin, line_number)?
            {
                write_reading_entries(
                    &mut generated_writer,
                    manual.word,
                    &readings,
                    line_number,
                    manual.weight,
                )?;
            } else {
                writeln!(
                    &mut generated_writer,
                    "{}\t{}\t{}",
                    manual.word,
                    manual.pinyin.replace(' ', "'"),
                    manual.weight
                )?;
            }
        } else {
            write_generated_entries(&mut generated_writer, checked_line, line_number, policy)?;
        }
    }

    let generated =
        String::from_utf8(generated).map_err(|error| invalid_data(error.to_string()))?;
    let entries: Vec<_> = generated.lines().collect();
    let sorted = sort_fcitx5_lines(&entries).map_err(invalid_data)?;
    for entry in sorted {
        writeln!(writer, "{entry}")?;
    }
    Ok(())
}

fn next_line(
    lines: &mut impl Iterator<Item = (usize, io::Result<String>)>,
) -> io::Result<Option<(usize, String)>> {
    lines
        .next()
        .map(|(index, line)| line.map(|line| (index + 1, line)))
        .transpose()
}

fn validate_line_length(line: &str, line_number: usize) -> io::Result<()> {
    if line.len() > MAX_ENTRY_BYTES {
        return Err(invalid_data(format!(
            "line {line_number}: entry is larger than the {MAX_ENTRY_BYTES}-byte safety limit"
        )));
    }
    Ok(())
}

fn write_generated_entries(
    writer: &mut impl Write,
    word: &str,
    line_number: usize,
    policy: &mut ReadingPolicy,
) -> io::Result<()> {
    let readings = policy.readings_for_word(word, line_number)?;
    write_reading_entries(writer, word, &readings, line_number, GENERATED_ENTRY_WEIGHT)
}

fn write_reading_entries(
    writer: &mut impl Write,
    word: &str,
    readings: &[Vec<&str>],
    line_number: usize,
    weight: &str,
) -> io::Result<()> {
    let mut combination_count = 1usize;
    for common in readings {
        combination_count = combination_count
            .checked_mul(common.len())
            .filter(|count| *count <= MAX_COMBINATIONS_PER_WORD)
            .ok_or_else(|| {
                invalid_data(format!(
                    "line {line_number}: {word:?} exceeds the limit of {MAX_COMBINATIONS_PER_WORD} pinyin combinations"
                ))
            })?;
    }

    let mut current = String::new();
    write_combinations(writer, word, readings, 0, &mut current, weight)
}

fn write_combinations(
    writer: &mut impl Write,
    word: &str,
    readings: &[Vec<&str>],
    index: usize,
    current: &mut String,
    weight: &str,
) -> io::Result<()> {
    if index == readings.len() {
        return writeln!(writer, "{word}\t{current}\t{weight}");
    }

    for reading in &readings[index] {
        let original_length = current.len();
        if !current.is_empty() {
            current.push('\'');
        }
        current.push_str(reading);
        write_combinations(writer, word, readings, index + 1, current, weight)?;
        current.truncate(original_length);
    }
    Ok(())
}

struct LimitedWriter<W> {
    inner: W,
    remaining: usize,
}

impl<W> LimitedWriter<W> {
    const fn new(inner: W, limit: usize) -> Self {
        Self {
            inner,
            remaining: limit,
        }
    }
}

impl<W: Write> Write for LimitedWriter<W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if buffer.len() > self.remaining {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                format!("output exceeds the {MAX_OUTPUT_BYTES}-byte safety limit"),
            ));
        }
        let written = self.inner.write(buffer)?;
        self.remaining -= written;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
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
    use std::io::Cursor;

    fn convert_text(input: &str) -> io::Result<String> {
        convert_text_with_rules(input, "{}")
    }

    fn convert_text_with_rules(input: &str, rules: &str) -> io::Result<String> {
        let mut output = Vec::new();
        let mut policy = ReadingPolicy::from_json(rules)?;
        convert(Cursor::new(input), &mut output, &mut policy)?;
        String::from_utf8(output).map_err(|error| invalid_data(error.to_string()))
    }

    #[test]
    fn converts_explicit_rime_pinyin_to_fcitx5_format() {
        let output = convert_text(
            "---\nname: test\n...\n薄缘的道与光与胤\tbao yuan de dao yu guang yu yin\t100\n薄缘的道与光与胤\tbo yuan de dao yu guang yu yin\t100\n",
        )
        .unwrap();
        let lines: Vec<_> = output.lines().collect();

        assert_eq!(lines.len(), 2);
        assert_eq!(
            lines[0],
            "薄缘的道与光与胤\tbao'yuan'de'dao'yu'guang'yu'yin\t100"
        );
        assert_eq!(
            lines[1],
            "薄缘的道与光与胤\tbo'yuan'de'dao'yu'guang'yu'yin\t100"
        );
    }

    #[test]
    fn generated_entries_use_at_most_two_readings_per_character() {
        let output = convert_text("---\n...\n长\n").unwrap();
        assert_eq!(output.lines().count(), 2);
    }

    #[test]
    fn converts_valid_manual_entry() {
        let output = convert_text("---\n...\n茜特菈莉\txi te la li\t100\n").unwrap();
        assert_eq!(output, "茜特菈莉\txi'te'la'li\t100\n");
    }

    #[test]
    fn rejects_unsupported_characters_instead_of_dropping_them() {
        let error = convert_text("---\n...\n派蒙2\n").unwrap_err();
        assert!(error.to_string().contains("unsupported character '2'"));
    }

    #[test]
    fn rejects_malformed_frontmatter() {
        let error = convert_text("name: test\n...\n派蒙\n").unwrap_err();
        assert!(
            error
                .to_string()
                .contains("expected opening YAML separator")
        );

        let error = convert_text("---\nname: test\n派蒙\n").unwrap_err();
        assert!(error.to_string().contains("no closing YAML separator"));
    }

    #[test]
    fn rejects_malformed_manual_entry() {
        let error = convert_text("---\n...\n派蒙\tpai meng\n").unwrap_err();
        assert!(error.to_string().contains("exactly three"));

        let error = convert_text("---\n...\n派蒙\tpai'meng\t100\n").unwrap_err();
        assert!(error.to_string().contains("separated by single spaces"));

        let error = convert_text("---\n...\n派蒙\tpai meng\t-1\n").unwrap_err();
        assert!(error.to_string().contains("non-negative integer"));
    }

    #[test]
    fn final_output_uses_shared_sorting_for_generated_and_manual_entries() {
        let output =
            convert_text("---\n...\n意义\n长\n阿\n异议\n薄缘\tbo yuan\t200\n薄缘\tbao yuan\t100\n")
                .unwrap();
        assert_eq!(
            output.lines().collect::<Vec<_>>(),
            [
                "阿\ta\t100",
                "阿\te\t100",
                "薄缘\tbao'yuan\t100",
                "薄缘\tbo'yuan\t200",
                "长\tchang\t100",
                "长\tzhang\t100",
                "意义\tyi'yi\t100",
                "异议\tyi'yi\t100",
            ]
        );
        let rows: Vec<_> = output.lines().collect();
        assert_eq!(sort_fcitx5_lines(&rows).unwrap(), rows);
    }

    #[test]
    fn example_rules_expand_characters_and_words_with_whitelist_precedence() {
        let mut policy =
            ReadingPolicy::from_json(include_str!("../tests/fixtures/pinyin_rules.example.json"))
                .unwrap();
        let mut output = Vec::new();
        convert(
            Cursor::new("---\n...\n莘莘\n若陀龙王\n若\n阿房宫赋\n阿房\n着\n哪\n长\n"),
            &mut output,
            &mut policy,
        )
        .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("阿房宫赋\te'pang'gong'fu\t100\n"));
        assert!(output.contains("阿房\ta'fang\t100\n"));
        assert!(output.contains("若陀龙王\tre'tuo'long'wang\t100\n"));
        assert!(output.contains("若陀龙王\truo'tuo'long'wang\t100\n"));
        assert!(output.contains("若\truo\t100\n"));
        assert!(output.contains("哪\tna\t100\n"));
        assert!(output.contains("长\tchang\t100\n"));
        assert!(output.contains("长\tzhang\t100\n"));
        let shen_rows: Vec<_> = output
            .lines()
            .filter(|line| line.starts_with("莘莘\t"))
            .collect();
        assert_eq!(
            shen_rows,
            [
                "莘莘\tshen'shen\t100",
                "莘莘\tshen'xin\t100",
                "莘莘\txin'shen\t100",
                "莘莘\txin'xin\t100",
            ]
        );
        let epang_rows: Vec<_> = output
            .lines()
            .filter(|line| line.starts_with("阿房宫赋\t"))
            .collect();
        assert_eq!(epang_rows, ["阿房宫赋\te'pang'gong'fu\t100"]);
        assert!(
            !output.lines().any(|line| line == "若\tre\t100"
                || line == "着\tzhuo\t100"
                || line == "哪\tne\t100")
        );
    }

    #[test]
    fn default_rules_do_not_enable_the_example_whitelist() {
        let output = convert_text("---\n...\n莘\n若陀\n阿房宫赋\n").unwrap();
        assert_eq!(
            output.lines().collect::<Vec<_>>(),
            [
                "阿房宫赋\ta'fang'gong'fu\t100",
                "阿房宫赋\te'fang'gong'fu\t100",
                "若陀\truo'tuo\t100",
                "莘\tshen\t100",
            ]
        );
    }

    #[test]
    fn explicit_manual_readings_keep_their_weight_and_do_not_expand() {
        let output = convert_text("---\n...\n阿房宫赋\te pang gong fu\t300\n").unwrap();
        assert_eq!(output, "阿房宫赋\te'pang'gong'fu\t300\n");
    }

    #[test]
    fn character_whitelist_also_reorganizes_manual_entries_and_preserves_other_syllables() {
        let output = convert_text_with_rules(
            "---\n...\n美露莘\tmei lu xin\t250\n",
            r#"{"whitelist":{"characters":{"莘":["xin","shen"]}}}"#,
        )
        .unwrap();
        assert_eq!(
            output.lines().collect::<Vec<_>>(),
            ["美露莘\tmei'lu'shen\t250", "美露莘\tmei'lu'xin\t250",]
        );
    }

    #[test]
    fn word_whitelist_reorganizes_multiple_manual_characters_before_blacklist() {
        let output = convert_text_with_rules(
            "---\n...\n阿房宫赋\ta fang gong fu\t300\n阿房\ta fang\t200\n",
            r#"{
                "whitelist":{"words":{"阿房宫赋":{"阿":["e"],"房":["pang"]}}},
                "blacklist":{"only":{"阿":["a"],"房":["fang"]}}
            }"#,
        )
        .unwrap();
        assert_eq!(
            output.lines().collect::<Vec<_>>(),
            ["阿房\ta'fang\t200", "阿房宫赋\te'pang'gong'fu\t300",]
        );
    }

    #[test]
    fn manual_entries_follow_blacklist_and_require_aligned_syllables_for_overrides() {
        let blacklist = r#"{"blacklist":{"characters":{"着":["zhuo"]}}}"#;
        let error = convert_text_with_rules("---\n...\n着\tzhuo\t100\n", blacklist).unwrap_err();
        assert!(error.to_string().contains("no permitted reading"));
        assert_eq!(
            convert_text_with_rules("---\n...\n着\tzhe\t200\n", blacklist).unwrap(),
            "着\tzhe\t200\n"
        );

        let error = convert_text_with_rules(
            "---\n...\n莘莘\txin\t100\n",
            r#"{"whitelist":{"characters":{"莘":["xin","shen"]}}}"#,
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("one pinyin syllable per character")
        );
    }

    #[test]
    fn manual_entries_keep_character_and_combination_limits() {
        let too_long = "长".repeat(MAX_CHARACTERS_PER_WORD + 1);
        let error = convert_text(&format!("---\n...\n{too_long}\tchang\t100\n")).unwrap_err();
        assert!(error.to_string().contains("characters"));

        let word = "长".repeat(20);
        let pinyin = ["chang"; 20].join(" ");
        let error = convert_text_with_rules(
            &format!("---\n...\n{word}\t{pinyin}\t100\n"),
            r#"{"whitelist":{"characters":{"长":["chang","zhang"]}}}"#,
        )
        .unwrap_err();
        assert!(error.to_string().contains("pinyin combinations"));
    }

    #[test]
    fn rejects_word_and_combination_limits_before_writing_output() {
        for (word, message) in [
            ("长".repeat(MAX_CHARACTERS_PER_WORD + 1), "characters"),
            ("长".repeat(20), "pinyin combinations"),
        ] {
            let mut output = Vec::new();
            let mut policy = ReadingPolicy::new(Rules::default()).unwrap();
            let error = convert(
                Cursor::new(format!("---\n...\n{word}\n")),
                &mut output,
                &mut policy,
            )
            .unwrap_err();
            assert!(error.to_string().contains(message));
            assert_eq!(output, Vec::<u8>::new());
        }
    }

    #[test]
    fn output_byte_limit_is_enforced_during_expansion() {
        let mut output = Vec::new();
        let mut limited = LimitedWriter::new(&mut output, 5);
        limited.write_all(b"12345").unwrap();
        assert_eq!(
            limited.write_all(b"6").unwrap_err().kind(),
            io::ErrorKind::FileTooLarge
        );
        assert_eq!(output, b"12345");
    }

    #[test]
    fn loads_default_sidecar_and_reports_explicit_missing_or_invalid_rules() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("input.dict.yaml");
        let sidecar = directory.path().join(RULES_FILE_NAME);
        assert!(load_reading_policy(&input, None).is_ok());
        assert!(load_reading_policy(&input, Some(&sidecar)).is_err());
        std::fs::write(
            &sidecar,
            r#"{"whitelist":{"characters":{"莘":["xin","shen"]}}}"#,
        )
        .unwrap();
        let mut policy = load_reading_policy(&input, None).unwrap();
        assert_eq!(
            policy.readings_for_word("莘", 1).unwrap(),
            vec![vec!["shen", "xin"]]
        );
        std::fs::write(&sidecar, "invalid json").unwrap();
        let error = load_reading_policy(&input, Some(&sidecar)).err().unwrap();
        assert!(error.to_string().contains("pinyin_rules.json"));
    }

    #[test]
    fn rejects_oversized_rules() {
        let directory = tempfile::tempdir().unwrap();
        let rules = directory.path().join("rules.json");
        std::fs::write(
            &rules,
            vec![b' '; usize::try_from(MAX_RULES_BYTES).unwrap() + 1],
        )
        .unwrap();
        let error = load_reading_policy(&directory.path().join("input.yaml"), Some(&rules))
            .err()
            .unwrap();
        assert!(error.to_string().contains("safety limit"));
    }

    #[test]
    fn failed_conversion_preserves_the_existing_output_and_input() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("input.dict.yaml");
        let output = directory.path().join(OUTPUT_FILE_NAME);
        let dictionary = "---\n...\n派蒙2\n";
        std::fs::write(&input, dictionary).unwrap();
        std::fs::write(&output, "previous output\n").unwrap();
        assert!(run([OsString::from("test"), input.clone().into_os_string()]).is_err());
        assert_eq!(std::fs::read_to_string(input).unwrap(), dictionary);
        assert_eq!(
            std::fs::read_to_string(output).unwrap(),
            "previous output\n"
        );
    }
}
