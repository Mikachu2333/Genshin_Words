use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use crate::types::*;

/// All path-related operations
impl GetPathInfo {
    /// Check if paths are files or directories
    ///
    /// ### Return Value
    /// Returns a tuple of two booleans `(path1 is file, path2 is file)`
    /// * `true` - Path points to a file
    /// * `false` - Path points to a directory
    pub fn if_file(&self) -> (bool, bool) {
        (self.path1.is_file(), self.path2.is_file())
    }

    /// Check if two paths are in the same parent directory
    ///
    /// This method is used to determine if two paths are in the same folder, which is important 
    /// for determining the safety of rename operations. If two paths are in the same directory, 
    /// certain rename operations may not need temporary files.
    ///
    /// ### Return Value
    /// * `true` - Both paths are in the same parent directory
    /// * `false` - Paths are in different parent directories
    pub fn if_same_dir(&self) -> bool {
        match (self.path1.parent(), self.path2.parent()) {
            (Some(parent1), Some(parent2)) => parent1 == parent2,
            _ => false,
        }
    }

    /// Detect if there is an inclusion relationship between two paths (parent-child directory issue)
    ///
    /// This method is used to determine if there is an inclusion relationship between two paths, 
    /// which is crucial for determining the rename order. When two directories have a parent-child 
    /// relationship, the rename order will directly affect the success of the operation.
    ///
    /// ### Return Value
    /// * `1` - path1 contains path2 (path1 is the parent or ancestor directory of path2)
    /// * `2` - path2 contains path1 (path2 is the parent or ancestor directory of path1)
    /// * `0` - No inclusion relationship
    pub fn if_root(&self) -> u8 {
        if Self::path_is_parent(&self.path1, &self.path2) {
            1 // path1 contains path2
        } else if Self::path_is_parent(&self.path2, &self.path1) {
            2 // path2 contains path1
        } else {
            0 // No inclusion relationship
        }
    }

    /// Helper function: Check if there is a parent-child directory relationship
    ///
    /// Determine if potential_parent is the parent or ancestor directory of potential_child
    ///
    /// ### Parameters
    /// * `potential_parent` - Potential parent directory path
    /// * `potential_child` - Potential child directory path
    ///
    /// ### Return Value
    /// * `true` - There is indeed a parent-child relationship
    /// * `false` - No parent-child relationship
    fn path_is_parent(potential_parent: &Path, potential_child: &Path) -> bool {
        // Try to determine the path of child relative to parent
        if let Ok(relative) = potential_child.strip_prefix(potential_parent) {
            !relative.as_os_str().is_empty()
        } else {
            false
        }
    }

    /// Get metadata information of file or directory
    ///
    /// Extract the file name (without suffix), extension, and parent directory path
    ///
    /// ### Parameters
    /// * `file_path` - File or directory path to process
    /// * `is_file` - Indicates if path is a file or directory
    ///
    /// ### Return Value
    /// Returns `MetadataCollection` structure containing metadata
    fn get_info(file_path: &Path, is_file: bool) -> MetadataCollection {
        // Closure function to extract strings, processing file names and extensions
        // If processing extension, add leading dot "."
        let get_string_closure = |original_result: &Option<&OsStr>, is_ext: bool| {
            match original_result {
                Some(i) => {
                    if is_ext {
                        // Whether calculating suffix, if so, add leading dot "."
                        ".".to_owned() + i.to_str().unwrap()
                    } else {
                        i.to_str().unwrap().to_string()
                    }
                }
                /*
                If not available, ignore
                Since verification has been completed earlier, if Err occurs here, 
                it is due to special file naming and does not affect subsequent operations.
                e.g. "C:\\.cargo\\.config", this file cannot get suffix, this folder also cannot get suffix
                */
                None => String::new(),
            }
        };

        if !is_file {
            // Process directory path
            MetadataCollection {
                name: {
                    // For directories, name includes stem and extension (if any)
                    get_string_closure(&file_path.file_stem(), false)
                        + get_string_closure(&file_path.extension(), true).as_ref()
                },
                ext: String::new(), // Directories have no extension
                parent_dir: {
                    match &file_path.parent() {
                        Some(i) => i.to_path_buf(),
                        None => PathBuf::new(),
                    }
                },
            }
        } else {
            // Process file path
            MetadataCollection {
                name: get_string_closure(&file_path.file_stem(), false),
                ext: get_string_closure(&file_path.extension(), true),
                parent_dir: {
                    match &file_path.parent() {
                        Some(i) => i.to_path_buf(),
                        None => PathBuf::new(),
                    }
                },
            }
        }
    }

    /// Collect metadata information of two paths
    ///
    /// ### Parameters
    /// * `is_file1` - Indicates if path1 is a file or directory
    /// * `is_file2` - Indicates if path2 is a file or directory
    ///
    /// ### Return Value
    /// Returns tuple containing two metadata collections `(path1 metadata, path2 metadata)`
    pub fn metadata_collect(
        &self,
        is_file1: bool,
        is_file2: bool,
    ) -> (MetadataCollection, MetadataCollection) {
        let metadata1 = GetPathInfo::get_info(&self.path1, is_file1);
        let metadata2 = GetPathInfo::get_info(&self.path2, is_file2);
        (metadata1, metadata2)
    }
}