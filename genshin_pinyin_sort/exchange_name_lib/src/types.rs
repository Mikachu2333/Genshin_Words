use std::{io, path::PathBuf};

/// Unique identifier for generating temporary filenames
/// This GUID is used to create temporary filenames to ensure no conflict with existing files
pub const GUID: &str = "5E702FA07C2FB332B76B";
pub const DEBUG_MODE: bool = cfg!(debug_assertions);

/// Store metadata information of file or directory
///
/// Contains file or directory name, extension, and parent directory path
#[derive(Default, Debug, Clone)]
pub struct MetadataCollection {
    /// File name or directory name (without extension)
    pub name: String,
    /// File extension (including leading dot "."), empty string for directories
    pub ext: String,
    /// Parent directory path
    pub parent_dir: PathBuf,
}

/// Store path information required for file renaming
///
/// Contains original path, new path, and temporary transition path
#[derive(Default, Debug, Clone)]
pub struct PrepareName {
    /// Original path of file or directory
    pub original_path: PathBuf,
    /// Target path after renaming
    pub new_path: PathBuf,
    /// Temporary path used during renaming process
    pub pre_path: PathBuf,
}

/// Store complete file information structure
///
/// Contains file existence status, type information, and path data required for renaming
#[derive(Default, Debug, Clone)]
pub struct FileInfos {
    /// Whether file or directory exists
    pub is_exist: bool,
    /// Is file (true) or directory (false)
    pub is_file: bool,
    /// File metadata information (name, extension, and parent directory)
    pub packed_info: MetadataCollection,
    /// Path information required for renaming
    pub exchange: PrepareName,
}

/// Structure for handling two paths, used for path checking and operations
#[derive(Default, Debug, Clone)]
pub struct GetPathInfo {
    /// First file or directory path
    pub path1: PathBuf,
    /// Second file or directory path
    pub path2: PathBuf,
}

/// Main structure for file name exchange
///
/// Contains complete information of two files for executing rename operations
#[derive(Default, Debug, Clone)]
pub struct NameExchange {
    /// Complete information of first file
    pub f1: FileInfos,
    /// Complete information of second file
    pub f2: FileInfos,
}

/// Error type used internally in rename process
#[derive(Debug, Clone)]
pub enum RenameError {
    PermissionDenied,
    AlreadyExists,
    NotExists,
    SamePath,
    InvalidPath(String),
    Unknown(String),
}

impl RenameError {
    /// Map internal errors to final return codes
    pub fn to_code(&self) -> i32 {
        match self {
            Self::NotExists => 1,
            Self::PermissionDenied => 2,
            Self::AlreadyExists => 3,
            Self::SamePath => 4,
            Self::InvalidPath(_) => 5,
            Self::Unknown(_) => 255,
        }
    }
}

impl std::fmt::Display for RenameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::AlreadyExists => write!(f, "File already exists"),
            Self::NotExists => write!(f, "File does not exist"),
            Self::SamePath => write!(f, "Two paths refer to the same file"),
            Self::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
            Self::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl From<io::Error> for RenameError {
    fn from(value: io::Error) -> Self {
        match value.kind() {
            io::ErrorKind::NotFound => RenameError::NotExists,
            io::ErrorKind::PermissionDenied => RenameError::PermissionDenied,
            io::ErrorKind::AlreadyExists => RenameError::AlreadyExists,
            _ => RenameError::Unknown(value.to_string()),
        }
    }
}
