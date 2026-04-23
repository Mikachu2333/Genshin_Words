use std::{
    env,
    path::{Path, PathBuf},
};

use crate::types::{GetPathInfo, NameExchange, RenameError, DEBUG_MODE};

/// Swap names of two files or directories
///
/// ### Parameters
/// * `path1`        - First file or directory path
/// * `path2`        - Second file or directory path
/// * `preserve_ext` - Should preserve file ext or exchange
///
/// ### Return Value
/// * `Ok(())` - Successfully swapped
/// * `Err(RenameError)` - Error information
pub fn exchange_paths(
    path1: PathBuf,
    path2: PathBuf,
    preserve_ext: bool,
) -> Result<(), RenameError> {
    let base_dir = resolve_base_dir()?;

    let (exists1, path1) = resolve_path(&path1, &base_dir)?;
    let (exists2, path2) = resolve_path(&path2, &base_dir)?;
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
        return Err(RenameError::SamePath);
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
        &exchange_info.f2.packed_info.ext,
        preserve_ext,
    );
    (
        exchange_info.f2.exchange.pre_path,
        exchange_info.f2.exchange.new_path,
    ) = NameExchange::make_name(
        &exchange_info.f2.packed_info.parent_dir,
        &exchange_info.f1.packed_info.name,
        &exchange_info.f2.packed_info.ext,
        &exchange_info.f1.packed_info.ext,
        preserve_ext,
    );

    let is_conflict = |new_path: &PathBuf| {
        new_path.exists()
            && *new_path != exchange_info.f1.exchange.original_path
            && *new_path != exchange_info.f2.exchange.original_path
    };

    if is_conflict(&exchange_info.f1.exchange.new_path)
        || is_conflict(&exchange_info.f2.exchange.new_path)
    {
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
    // Prefer current working directory over executable directory
    if let Ok(cwd) = env::current_dir() {
        return Ok(cwd);
    }

    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            return Ok(parent.to_path_buf());
        }
    }

    Err(RenameError::Unknown(
        "Failed to resolve working directory".to_string(),
    ))
}

/// Resolve and normalize path
///
/// ### Parameters
/// * `path` - Original path
/// * `base_dir` - Base directory path
///
/// ### Return Value
/// * `Ok((bool, PathBuf))` - Tuple of (whether path exists, normalized path)
/// * `Err(RenameError)` - Path resolution failure (e.g. invalid UTF-8, missing env var)
pub fn resolve_path(path: &Path, base_dir: &Path) -> Result<(bool, PathBuf), RenameError> {
    if *path == *"" {
        return Ok((false, path.to_path_buf()));
    }

    let mut path = path.to_path_buf();

    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};

        path = {
            let temp = path
                .to_str()
                .ok_or_else(|| {
                    RenameError::InvalidPath(format!(
                        "Path contains invalid UTF-8: {}",
                        path.display()
                    ))
                })?
                .replace("/", "\\");
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
                } else {
                    return Err(RenameError::InvalidPath(
                        "USERPROFILE environment variable is not set, cannot expand '~'"
                            .to_string(),
                    ));
                }
            } else if path.starts_with(".") {
                if let Ok(remaining) = path.strip_prefix(".\\") {
                    path = base_dir.join(remaining);
                } else {
                    path = base_dir.join(&path);
                }
            } else {
                path = base_dir.join(path);
            }
        }
    }

    #[cfg(not(windows))]
    {
        path = {
            let temp = path
                .to_str()
                .ok_or_else(|| {
                    RenameError::InvalidPath(format!(
                        "Path contains invalid UTF-8: {}",
                        path.display()
                    ))
                })?
                .replace("\\", "/");
            PathBuf::from(temp)
        };

        if !path.is_absolute() {
            if path.starts_with("~") {
                if let Ok(home_dir) = std::env::var("HOME") {
                    let mut new_path = PathBuf::from(home_dir);
                    let remaining = path.strip_prefix("~/").ok();
                    if let Some(rem) = remaining {
                        new_path.push(rem);
                        path = new_path;
                    } else if *path == *"~" {
                        path = new_path;
                    } else {
                        path = base_dir.join(path);
                    }
                } else {
                    return Err(RenameError::InvalidPath(
                        "HOME environment variable is not set, cannot expand '~'".to_string(),
                    ));
                }
            } else if path.starts_with(".") {
                if let Ok(remaining) = path.strip_prefix("./") {
                    path = base_dir.join(remaining);
                } else {
                    path = base_dir.join(path);
                }
            } else {
                path = base_dir.join(path);
            }
        }
    }
    if DEBUG_MODE {
        println!("Checked Path: {}", &path.display());
    }

    let canonical = path.canonicalize();
    match canonical {
        Ok(x) => Ok((x.exists(), x)),
        Err(e) => {
            eprintln!("{}", e);
            Ok((path.exists(), path))
        }
    }
}
