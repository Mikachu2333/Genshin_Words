use std::path::PathBuf;

use file_rename::NameExchange;
use path_checkout::GetPathInfo;
mod file_rename;
mod path_checkout;

#[no_mangle]
/// # Safety
/// 最终暴露的执行函数，传入两个路径String，返回一个u8
///
/// 0 => Success，1 => No Exist
///
/// 2 => Permission Denied，3 => New File Already Exists
///
/// 255 => UNKNOWN ERROR
pub fn exchange(path1: String, path2: String) -> i32 {
    let binding = std::env::current_exe().unwrap();
    let exe_dir = binding.parent().unwrap();

    let mut all_infos = NameExchange::new();

    // 用于校验文件夹路径最后是否为斜杠与双引号的闭包
    let path_check = |s: String| {
        let temp = s
            .trim()
            .trim_matches(['\'', '"', '\\', '\'', '/'])
            .replace("\\", "/")
            .replace("//", "/");
        PathBuf::from(&temp)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(&temp))
    };
    let mut packed_path = GetPathInfo {
        path1: path_check(path1),
        path2: path_check(path2),
    };

    (all_infos.f1.is_exist, all_infos.f2.is_exist) = (packed_path).if_exist(exe_dir);
    if (!all_infos.f1.is_exist) || (!all_infos.f2.is_exist) {
        return 1_i32;
    }
    if packed_path.path1 == packed_path.path2 {
        return 2_i32;
    }
    all_infos.f1.exchange.original_path = packed_path.path1.clone();
    all_infos.f2.exchange.original_path = packed_path.path2.clone();

    (all_infos.f1.is_file, all_infos.f2.is_file) = packed_path.if_file();

    (all_infos.f1.packed_info, all_infos.f2.packed_info) =
        packed_path.metadata_collect(all_infos.f1.is_file, all_infos.f2.is_file);

    (
        all_infos.f1.exchange.pre_path,
        all_infos.f1.exchange.new_path,
    ) = NameExchange::make_name(
        &all_infos.f1.packed_info.parent_dir,
        &all_infos.f2.packed_info.name,
        &all_infos.f1.packed_info.ext,
    );
    (
        all_infos.f2.exchange.pre_path,
        all_infos.f2.exchange.new_path,
    ) = NameExchange::make_name(
        &all_infos.f2.packed_info.parent_dir,
        &all_infos.f1.packed_info.name,
        &all_infos.f2.packed_info.ext,
    );

    let mut packed_path_new = GetPathInfo {
        path1: all_infos.f1.exchange.new_path.clone(),
        path2: all_infos.f2.exchange.new_path.clone(),
    };
    let (exist_new_1, exist_new_2) = GetPathInfo::if_exist(&mut packed_path_new, exe_dir);
    let same_dir = GetPathInfo::if_same_dir(&packed_path_new);
    if !same_dir && (exist_new_1 || exist_new_2) {
        //不能因为rename函数里面有就删了……
        /*
        println!(
            "same:{}\tnew1:{}\tnew2:{}",
            same_dir, exist_new_1, exist_new_2
        );
        */
        return 3_i32;
    }

    //1 -> parent1, 2 -> parent2
    let mode = packed_path.if_root();
/*
    dbg!(
        //test
        all_infos.f1.packed_info.parent_dir.display(),
        &all_infos.f1.packed_info.name,
        &all_infos.f1.packed_info.ext
    );
    dbg!(
        all_infos.f2.packed_info.parent_dir.display(),
        &all_infos.f2.packed_info.name,
        &all_infos.f2.packed_info.ext
    );
    dbg!(mode);
 */
    match (all_infos.f1.is_file, all_infos.f2.is_file) {
        (true, true) => NameExchange::rename_each(&all_infos, false, true),
        (false, false) => {
            // 都是目录，检查包含关系
            match mode {
                1 => NameExchange::rename_each(&all_infos, true, false),
                2 => NameExchange::rename_each(&all_infos, true, true),
                _ => NameExchange::rename_each(&all_infos, false, true),
            }
        }
        (true, false) => {
            if mode == 2 {
                NameExchange::rename_each(&all_infos, true, true)
            } else {
                NameExchange::rename_each(&all_infos, false, true)
            }
        }
        (false, true) => {
            if mode == 1 {
                NameExchange::rename_each(&all_infos, true, false)
            } else {
                NameExchange::rename_each(&all_infos, false, false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    fn clear_olds() -> (PathBuf, PathBuf) {
        let file1 = Path::new(r"D:\Desktop\f\s\2.txt");
        let file2 = Path::new(r"D:\Desktop\f");
        return (file1.to_path_buf(), file2.to_path_buf());
    }

    #[test]
    fn it_works() {
        let (file1, file2) = clear_olds();
        // 0 => Success，1 => No Exist
        // 2 => Permission Denied，3 => New File Already Exists
        let trans = |s: PathBuf| s.to_str().unwrap().to_string();
        let test_path1 = trans(file1);
        let test_path2 = trans(file2);

        super::exchange(test_path1, test_path2);
    }
}
