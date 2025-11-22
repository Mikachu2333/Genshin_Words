pub const DEBUG_MODE: bool = cfg!(debug_assertions);

pub struct VersionInfo {
    pub a: String,
    pub b: String,
    pub c: String,
}
impl VersionInfo {
    #[allow(dead_code)]
    pub fn default() -> Self {
        let temp = "0";
        Self {
            a: temp.to_string(),
            b: temp.to_string(),
            c: temp.to_string(),
        }
    }
    pub fn from_str(original: &str) -> Self {
        let binding = original
            .trim()
            .to_lowercase()
            .replace("version", "")
            .replace("ver", "");
        let parsed = binding
            .splitn(4, &['.', '-', ' ', '(', ')', '[', ']', '~', '_'])
            .collect::<Vec<&str>>();

        if DEBUG_MODE {
            dbg!("Version {}", &parsed);
        }

        let patch_version = parsed[2].parse::<usize>();
        let patch_result = if patch_version.is_err() {
            1_usize
        } else {
            patch_version.unwrap() + 1
        };

        Self {
            a: parsed[0].to_string(),
            b: parsed[1].to_string(),
            c: patch_result.to_string(),
        }
    }
    pub fn to_str(self) -> String {
        let format_symbols = "()".chars().collect::<Vec<char>>();
        format!(
            "{}.{}{}{}{}",
            self.a, self.b, format_symbols[0], self.c, format_symbols[1]
        )
    }
}
