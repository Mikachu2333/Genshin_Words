use std::path::{Path, PathBuf};

use crate::types::*;

/// Main rename logic integration
impl NameExchange {
    /// Initialize structure for storing all information
    ///
    /// Create a new NameExchange instance with two default initialized FileInfos
    pub fn new() -> NameExchange {
        NameExchange {
            f1: FileInfos {
                ..Default::default()
            },
            f2: FileInfos {
                ..Default::default()
            },
        }
    }

    /// Get temporary filename and renamed filename
    ///
    /// Generate temporary file path and final file path based on directory path, filename, and extension
    ///
    /// ### Parameters
    /// * `dir` - Directory path where file is located
    /// * `other_name` - Target filename (without extension)
    /// * `ext` - File extension (including leading dot ".")
    ///
    /// ### Return Value
    /// Returns tuple `(temporary file path, final file path)`
    pub fn make_name(
        dir: &Path,
        other_name: impl ToString,
        ext: impl ToString,
    ) -> (PathBuf, PathBuf) {
        let other_name = other_name.to_string();
        let ext = ext.to_string();
        let mut temp_path = dir.to_path_buf();
        let mut final_path = dir.to_path_buf();

        // Arbitrary long string for distinction
        let mut temp_name = crate::types::GUID.to_string();
        temp_name.push_str(&ext);
        temp_path.push(temp_name);

        let final_component = if ext.is_empty() {
            other_name
        } else {
            format!("{}{}", other_name, ext)
        };

        if !final_component.is_empty() {
            final_path.push(final_component);
        }

        (temp_path, final_path)
    }

    /// Rename execution part
    ///
    /// Execute rename operation based on file type and nesting relationship
    ///
    /// ### Parameters
    /// * `is_nested` - Whether there is a nesting relationship (such as parent-child directories)
    /// * `file1_first` - Whether to rename the first file first
    ///
    /// ### Return Value
    /// Returns `Ok(())` for success, `Err(RenameError)` for corresponding failure reason
    pub fn rename_each(&self, is_nested: bool, file1_first: bool) -> Result<(), RenameError> {
        // Prepare path variables according to rename order
        let mut path1 = self.f2.exchange.original_path.clone();
        let mut final_name1 = self.f2.exchange.new_path.clone();
        let mut path2 = self.f1.exchange.original_path.clone();
        let mut final_name2 = self.f1.exchange.new_path.clone();
        let mut tmp_name2 = self.f1.exchange.pre_path.clone();
        if file1_first {
            path1 = self.f1.exchange.original_path.clone();
            final_name1 = self.f1.exchange.new_path.clone();
            path2 = self.f2.exchange.original_path.clone();
            final_name2 = self.f2.exchange.new_path.clone();
            tmp_name2 = self.f2.exchange.pre_path.clone();
        }

        //1 first
        if is_nested {
            // If there is a nesting relationship (parent-child directories or files),
            // rename directly in order
            // Do not use temporary files, as using temporary files in nesting relationships
            // may cause path issues
            Self::handle_rename(&path1, &final_name1)?;
            Self::handle_rename(&path2, &final_name2)?;
            Ok(())
        } else {
            // No nesting relationship: use temporary files for safe swapping
            // 1. Rename the second file to temporary file
            // 2. Rename the first file to final name
            // 3. Rename the temporary file to final name
            Self::handle_rename(&path2, &tmp_name2)?;
            Self::handle_rename(&path1, &final_name1)?;
            Self::handle_rename(&tmp_name2, &final_name2)?;
            Ok(())
        }
    }

    /// Handle single rename operation and process possible errors
    ///
    /// ### Parameters
    /// * `from` - Original file path
    /// * `to` - Target file path
    ///
    /// ### Return Value
    /// Returns `Ok(())` for success, `Err(RenameError)` for specific error
    fn handle_rename(from: &Path, to: &Path) -> Result<(), RenameError> {
        match std::fs::rename(from, to) {
            Ok(_) => Ok(()),
            Err(e) => Err(RenameError::from(e)),
        }
    }
}
