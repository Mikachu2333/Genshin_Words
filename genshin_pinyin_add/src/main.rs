use pinyin::ToPinyinMulti;
use std::{
    env,
    error::Error,
    ffi::OsString,
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};
use tempfile::NamedTempFile;

const OUTPUT_FILE_NAME: &str = "yuanshen_fcitx5.txt";
const GENERATED_ENTRY_WEIGHT: u64 = 100;
const COMMON_READINGS_PER_CHARACTER: usize = 2;
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
    let path_in = match (args.next(), args.next()) {
        (Some(path), None) => PathBuf::from(path),
        _ => {
            return Err(invalid_input(format!(
                "GenShin Impact Word to Pinyin {}\nUsage: \"{}\" <FullFilePath>",
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
        convert(BufReader::new(input), &mut writer)?;
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

fn convert(reader: impl BufRead, writer: &mut impl Write) -> io::Result<()> {
    let mut writer = LimitedWriter::new(writer, MAX_OUTPUT_BYTES);
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
            let manual = parse_manual_entry(checked_line, line_number)?;
            writeln!(
                &mut writer,
                "{}\t{}\t{}",
                manual.word,
                manual.pinyin.replace(' ', "'"),
                manual.weight
            )?;
        } else {
            write_generated_entries(&mut writer, checked_line, line_number)?;
        }
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

struct ManualEntry<'a> {
    word: &'a str,
    pinyin: &'a str,
    weight: &'a str,
}

fn parse_manual_entry(entry: &str, line_number: usize) -> io::Result<ManualEntry<'_>> {
    let fields: Vec<_> = entry.split('\t').collect();
    if fields.len() != 3 || fields.iter().any(|field| field.is_empty()) {
        return Err(invalid_data(format!(
            "line {line_number}: manual entry must contain exactly three non-empty tab-separated fields"
        )));
    }
    validate_rime_pinyin(fields[1], line_number)?;
    fields[2].parse::<u64>().map_err(|_| {
        invalid_data(format!(
            "line {line_number}: manual entry weight must be a non-negative integer"
        ))
    })?;
    Ok(ManualEntry {
        word: fields[0],
        pinyin: fields[1],
        weight: fields[2],
    })
}

fn validate_rime_pinyin(pinyin: &str, line_number: usize) -> io::Result<()> {
    if pinyin.split(' ').any(|syllable| {
        syllable.is_empty() || !syllable.bytes().all(|byte| byte.is_ascii_lowercase())
    }) {
        return Err(invalid_data(format!(
            "line {line_number}: Rime pinyin must contain lowercase ASCII syllables separated by single spaces"
        )));
    }
    Ok(())
}

fn write_generated_entries(
    writer: &mut impl Write,
    word: &str,
    line_number: usize,
) -> io::Result<()> {
    let character_count = word.chars().count();
    if character_count > MAX_CHARACTERS_PER_WORD {
        return Err(invalid_data(format!(
            "line {line_number}: word exceeds the limit of {MAX_CHARACTERS_PER_WORD} characters"
        )));
    }

    let mut readings = Vec::with_capacity(character_count);
    let mut combination_count = 1usize;

    for (character_index, character) in word.chars().enumerate() {
        let multi = character.to_pinyin_multi().ok_or_else(|| {
            invalid_data(format!(
                "line {line_number}: unsupported character {character:?} at character {}",
                character_index + 1
            ))
        })?;
        // The pinyin data is ordered by common usage. This project intentionally keeps
        // at most the first two readings instead of expanding every rare pronunciation.
        let mut common: Vec<_> = multi
            .into_iter()
            .take(COMMON_READINGS_PER_CHARACTER)
            .map(pinyin::Pinyin::plain)
            .collect();
        common.sort_unstable();
        common.dedup();

        combination_count = combination_count
            .checked_mul(common.len())
            .filter(|count| *count <= MAX_COMBINATIONS_PER_WORD)
            .ok_or_else(|| {
                invalid_data(format!(
                    "line {line_number}: {word:?} exceeds the limit of {MAX_COMBINATIONS_PER_WORD} pinyin combinations"
                ))
            })?;
        readings.push(common);
    }

    let mut current = String::new();
    write_combinations(writer, word, &readings, 0, &mut current)
}

fn write_combinations(
    writer: &mut impl Write,
    word: &str,
    readings: &[Vec<&str>],
    index: usize,
    current: &mut String,
) -> io::Result<()> {
    if index == readings.len() {
        return writeln!(writer, "{word}\t{current}\t{GENERATED_ENTRY_WEIGHT}");
    }

    for reading in &readings[index] {
        let original_length = current.len();
        if !current.is_empty() {
            current.push('\'');
        }
        current.push_str(reading);
        write_combinations(writer, word, readings, index + 1, current)?;
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
        let mut output = Vec::new();
        convert(Cursor::new(input), &mut output)?;
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
        let output = convert_text("---\n...\n还\n").unwrap();
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
}
