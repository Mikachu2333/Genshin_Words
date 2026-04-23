use std::ffi::{c_char, CStr};
use std::path::{Path, PathBuf};

mod exchange;
mod file_rename;
mod path_checkout;
mod types;

use crate::exchange::{exchange_paths, resolve_path};
pub use crate::types::RenameError;

#[no_mangle]
/// # Safety
/// C interface function for swapping names of two files or directories
///
/// ### Parameters
/// * `path1` - First file or directory path (C string pointer)
/// * `path2` - Second file or directory path (C string pointer)
/// * `preserve_ext` - If true, keep each file's own extension and swap only stems
///
/// ### Return Value
/// * `0` - Success
/// * `1` - File does not exist
/// * `2` - Permission denied
/// * `3` - Target file already exists
/// * `4` - Two paths refer to the same file
/// * `5` - Invalid path (e.g. non-UTF-8)
/// * `255` - Unknown error
pub unsafe extern "C" fn exchange(
    path1: *const c_char,
    path2: *const c_char,
    preserve_ext: bool,
) -> i32 {
    unsafe { convert_inputs(path1, path2) }
        .and_then(|(path1, path2)| exchange_paths(path1, path2, preserve_ext))
        .map(|_| 0)
        .unwrap_or_else(|err| err.to_code())
}

/// Rust interface function for swapping names of two files or directories
///
/// ### Parameters
/// * `path1` - First file or directory path
/// * `path2` - Second file or directory path
/// * `preserve_ext` - If true, keep each file's own extension and swap only stems
///
/// ### Return Value
/// * `Ok(())` - Success
/// * `Err(RenameError)` - Error information
pub fn exchange_rs(
    path1: &Path,
    path2: &Path,
    preserve_ext: bool,
) -> Result<(), RenameError> {
    exchange_paths(path1.to_path_buf(), path2.to_path_buf(), preserve_ext)
}

/// Resolve and normalize path
///
/// ### Parameters
/// * `path` - Original path
/// * `base_dir` - Base directory path
///
/// ### Return Value
/// * `Ok((bool, PathBuf))` - Tuple of (is_path_exists, normalized_path)
/// * `Err(RenameError)` - Path resolution failure
pub fn resolve_path_rs(
    path: &Path,
    base_dir: &Path,
) -> Result<(bool, PathBuf), RenameError> {
    resolve_path(path, base_dir)
}

unsafe fn convert_inputs(
    path1: *const c_char,
    path2: *const c_char,
) -> Result<(PathBuf, PathBuf), RenameError> {
    let path1 = ptr_to_path(path1)?;
    let path2 = ptr_to_path(path2)?;
    Ok((path1, path2))
}

unsafe fn ptr_to_path(ptr: *const c_char) -> Result<PathBuf, RenameError> {
    if ptr.is_null() {
        return Err(RenameError::NotExists);
    }

    let c_str = CStr::from_ptr(ptr);
    let raw = c_str.to_string_lossy();
    let sanitized = sanitize_input(raw.as_ref());

    if sanitized.is_empty() {
        return Err(RenameError::NotExists);
    }

    Ok(PathBuf::from(sanitized))
}

fn sanitize_input(input: &str) -> String {
    input
        .trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        io::Write,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(case_name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let mut path = std::env::temp_dir();
            path.push(format!(
                "name_exchanger_rs_{}_{}_{}",
                case_name,
                std::process::id(),
                unique
            ));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn join(&self, file_name: &str) -> PathBuf {
            self.path.join(file_name)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write_text(path: &Path, content: &str) {
        let mut file = File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    fn read_text(path: &Path) -> String {
        fs::read_to_string(path).unwrap()
    }

    #[test]
    fn exchange_rs_with_preserve_ext_false_swaps_full_names() {
        let dir = TestDir::new("swap_full_name");
        let file1 = dir.join("alpha.ext1");
        let file2 = dir.join("beta.ext2");
        write_text(&file1, "A");
        write_text(&file2, "B");

        super::exchange_rs(&file1, &file2, false).unwrap();

        assert!(file1.exists());
        assert!(file2.exists());
        assert_eq!(read_text(&file1), "B");
        assert_eq!(read_text(&file2), "A");
    }

    #[test]
    fn exchange_rs_with_preserve_ext_true_keeps_extensions() {
        let dir = TestDir::new("keep_extension");
        let file1 = dir.join("alpha.ext1");
        let file2 = dir.join("beta.ext2");
        write_text(&file1, "A");
        write_text(&file2, "B");

        super::exchange_rs(&file1, &file2, true).unwrap();

        let new_file1 = dir.join("beta.ext1");
        let new_file2 = dir.join("alpha.ext2");

        assert!(!file1.exists());
        assert!(!file2.exists());
        assert!(new_file1.exists());
        assert!(new_file2.exists());
        assert_eq!(read_text(&new_file1), "A");
        assert_eq!(read_text(&new_file2), "B");
    }

    #[test]
    fn exchange_rs_same_path_returns_error() {
        let dir = TestDir::new("same_path");
        let file = dir.join("same.ext");
        write_text(&file, "X");

        let result = super::exchange_rs(&file, &file, true);
        assert!(matches!(result, Err(super::types::RenameError::SamePath)));
    }
}
