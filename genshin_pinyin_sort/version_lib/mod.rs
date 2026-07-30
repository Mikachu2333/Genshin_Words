#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VersionInfo {
    major: usize,
    minor: usize,
    patch: usize,
}

impl VersionInfo {
    /// Parses a three-component version and increments its patch component.
    ///
    /// # Errors
    ///
    /// Returns an error when the version shape or a numeric component is invalid,
    /// or when incrementing the patch component would overflow.
    pub fn parse(original: &str) -> Result<Self, String> {
        let normalized = original
            .trim()
            .to_ascii_lowercase()
            .replace("version", "")
            .replace("ver", "");
        let parts: Vec<_> = normalized
            .split(|character: char| {
                matches!(
                    character,
                    '.' | '-' | ' ' | '(' | ')' | '[' | ']' | '~' | '_'
                )
            })
            .filter(|part| !part.is_empty())
            .collect();
        if parts.len() != 3 {
            return Err(format!(
                "version must contain exactly major, minor, and patch components: {original:?}"
            ));
        }

        let major = parse_component(parts[0], "major")?;
        let minor = parse_component(parts[1], "minor")?;
        let patch = parse_component(parts[2], "patch")?
            .checked_add(1)
            .ok_or_else(|| "patch version overflow".to_string())?;
        Ok(Self {
            major,
            minor,
            patch,
        })
    }

    #[must_use]
    pub fn to_str(&self) -> String {
        format!("{}.{}({})", self.major, self.minor, self.patch)
    }
}

fn parse_component(component: &str, name: &str) -> Result<usize, String> {
    component
        .parse::<usize>()
        .map_err(|error| format!("invalid {name} version component {component:?}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_increments_supported_version() {
        let version = VersionInfo::parse("6.8(5)").unwrap();
        assert_eq!(version.to_str(), "6.8(6)");
    }

    #[test]
    fn rejects_missing_or_extra_components() {
        assert!(VersionInfo::parse("6.8").is_err());
        assert!(VersionInfo::parse("6.8.5.1").is_err());
        assert!(VersionInfo::parse("6.x(5)").is_err());
    }

    #[test]
    fn rejects_patch_overflow() {
        let input = format!("1.2({})", usize::MAX);
        assert_eq!(
            VersionInfo::parse(&input).unwrap_err(),
            "patch version overflow"
        );
    }
}
