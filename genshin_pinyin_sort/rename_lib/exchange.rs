use std::{
    env,
    path::{Path, PathBuf},
};

use crate::types::{GetPathInfo, NameExchange, RenameError, DEBUG_MODE};

/// Swap names of two files or directories
///
/// ### Parameters
/// * `path1` - First file or directory path
/// * `path2` - Second file or directory path
///
/// ### Return Value
/// * `Ok(())` - Successfully swapped
/// * `Err(RenameError)` - Error information
pub fn exchange_paths(path1: PathBuf, path2: PathBuf) -> Result<(), RenameError> {
    let base_dir = resolve_base_dir()?;

    let (exists1, path1) = resolve_path(&path1, &base_dir);
    let (exists2, path2) = resolve_path(&path2, &base_dir);
    if DEBUG_MODE {
        dbg!(exists1, &path1, exists2, &path2);
    }
    if !exists1 || !exists2 {
        if !exists1 {
            eprintln!("{}", path1.display());
        }
        if !exists2 {
            eprintln!("{}", path2.display());
        }
        return Err(RenameError::NotExists);
    }

    if path1 == path2 {
        return Err(RenameError::AlreadyExists);
    }

    let mut exchange_info = NameExchange::new();
    exchange_info.f1.is_exist = true;
    exchange_info.f2.is_exist = true;

    let original_paths = GetPathInfo { path1, path2 };

    (exchange_info.f1.is_file, exchange_info.f2.is_file) = original_paths.if_file();
    (exchange_info.f1.packed_info, exchange_info.f2.packed_info) =
        original_paths.metadata_collect(exchange_info.f1.is_file, exchange_info.f2.is_file);

    exchange_info.f1.exchange.original_path = original_paths.path1.clone();
    exchange_info.f2.exchange.original_path = original_paths.path2.clone();

    (
        exchange_info.f1.exchange.pre_path,
        exchange_info.f1.exchange.new_path,
    ) = NameExchange::make_name(
        &exchange_info.f1.packed_info.parent_dir,
        &exchange_info.f2.packed_info.name,
        &exchange_info.f1.packed_info.ext,
    );
    (
        exchange_info.f2.exchange.pre_path,
        exchange_info.f2.exchange.new_path,
    ) = NameExchange::make_name(
        &exchange_info.f2.packed_info.parent_dir,
        &exchange_info.f1.packed_info.name,
        &exchange_info.f2.packed_info.ext,
    );

    let new_path_conflict_1 = exchange_info.f1.exchange.new_path.exists();
    let new_path_conflict_2 = exchange_info.f2.exchange.new_path.exists();
    let same_parent = GetPathInfo {
        path1: exchange_info.f1.exchange.new_path.clone(),
        path2: exchange_info.f2.exchange.new_path.clone(),
    }
    .if_same_dir();

    if !same_parent && (new_path_conflict_1 || new_path_conflict_2) {
        return Err(RenameError::AlreadyExists);
    }

    let mode = original_paths.if_root();

    match (exchange_info.f1.is_file, exchange_info.f2.is_file) {
        (true, true) => NameExchange::rename_each(&exchange_info, false, true),
        (false, false) => match mode {
            1 => NameExchange::rename_each(&exchange_info, true, false),
            2 => NameExchange::rename_each(&exchange_info, true, true),
            _ => NameExchange::rename_each(&exchange_info, false, true),
        },
        (true, false) => {
            if mode == 2 {
                NameExchange::rename_each(&exchange_info, true, true)
            } else {
                NameExchange::rename_each(&exchange_info, false, true)
            }
        }
        (false, true) => {
            if mode == 1 {
                NameExchange::rename_each(&exchange_info, true, false)
            } else {
                NameExchange::rename_each(&exchange_info, false, false)
            }
        }
    }
}

/// Resolve base directory path
///
/// ### Return Value
/// * `Ok(PathBuf)` - Base directory path
/// * `Err(RenameError)` - Resolution failure
fn resolve_base_dir() -> Result<PathBuf, RenameError> {
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            return Ok(parent.to_path_buf());
        }
    }

    env::current_dir().map_err(|err| {
        RenameError::Unknown(format!("Failed to resolve working directory: {}", err))
    })
}

/// Resolve and normalize path
///
/// ### Parameters
/// * `path` - Original path
/// * `base_dir` - Base directory path
///
/// ### Return Value
/// Returns tuple `(whether path exists, normalized path)`
pub fn resolve_path(path: &Path, base_dir: &Path) -> (bool, PathBuf) {
    if *path == *"" {
        return (false, path.to_path_buf());
    }

    let mut path = path.to_path_buf();

    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};

        path = {
            let temp = path.to_str().unwrap_or("").replace("/", "\\");
            PathBuf::from(temp)
        };

        let is_absolute = {
            let mut components = path.components();
            if let Some(Component::Prefix(prefix_component)) = components.next() {
                let has_root_dir = matches!(components.next(), Some(Component::RootDir));
                if DEBUG_MODE {
                    dbg!(has_root_dir);
                }
                if !has_root_dir {
                    false
                } else {
                    if DEBUG_MODE {
                        dbg!(prefix_component.kind());
                    }

                    matches!(
                        prefix_component.kind(),
                        Prefix::VerbatimUNC(..)
                            | Prefix::UNC(..)
                            | Prefix::VerbatimDisk(..)
                            | Prefix::Disk(_)
                            | Prefix::DeviceNS(..)
                            | Prefix::Verbatim(_)
                    )
                }
            } else {
                path.is_absolute()
            }
        };

        if !is_absolute {
            if path.starts_with("~") {
                if let Ok(home_dir) = std::env::var("USERPROFILE") {
                    let mut new_path = PathBuf::from(home_dir);
                    let remaining = path.strip_prefix("~/").ok();
                    if let Some(rem) = remaining {
                        new_path.push(rem);
                        path = new_path;
                    } else if *path == *"~" {
                        path = new_path;
                    } else {
                        // "~something"
                        path = base_dir.join(path);
                    }
                }
            } else if path.starts_with(".") {
                let remaining = path.strip_prefix(".\\").ok();
                path = base_dir.join(remaining.unwrap());
            } else {
                path = base_dir.join(path);
            }
        }
    }

    #[cfg(not(windows))]
    {
        path = {
            let temp = path.to_str().unwrap_or("").replace("\\", "/");
            PathBuf::from(temp)
        };

        if !path.is_absolute() {
            if path.starts_with("~") {
                if let Ok(home_dir) = std::env::var("HOME") {
                    let mut new_path = PathBuf::from(home_dir);
                    if let Some(remaining) = path.strip_prefix("~/") {
                        new_path.push(remaining);
                    } else if *path== *"~" {
                        // Just "~", so it's the home directory
                    }
                    path = new_path;
                }
            } else if path.starts_with(".") {
                let remaining = path.strip_prefix("./").ok();
                path = base_dir.join(remaining.unwrap());
            } else {
                path = base_dir.join(path);
            }
        }
    }
    if DEBUG_MODE {
        dbg!("Path Final: {}", &path.display());
    }

    let canonical = path.canonicalize();
    match canonical {
        Ok(x) => (x.exists(), x),
        Err(e) => {
            eprintln!("{}", e);
            (path.exists(), path)
        }
    }
}
