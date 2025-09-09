use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

/// 存储文件或目录的元数据信息
///
/// 包含文件或目录的名称、扩展名和父目录路径
#[derive(Debug)]
pub struct MetadataCollection {
    /// 文件名或目录名（不包含扩展名）
    pub name: String,
    /// 文件扩展名（包含前导点"."），目录为空字符串
    pub ext: String,
    /// 父目录的路径
    pub parent_dir: PathBuf,
}

impl Default for MetadataCollection {
    /// 创建默认的元数据集合，所有字段为空
    fn default() -> Self {
        Self {
            name: "".to_owned(),
            ext: "".to_owned(),
            parent_dir: PathBuf::new(),
        }
    }
}

/// 处理两个路径的结构体，用于路径检查和操作
#[derive(Debug)]
pub struct GetPathInfo {
    /// 第一个文件或目录路径
    pub path1: PathBuf,
    /// 第二个文件或目录路径
    pub path2: PathBuf,
}

/// 所有路径相关的操作
impl GetPathInfo {
    /// 校验路径是否存在，并处理相对路径
    ///
    /// 如果路径是相对路径，会尝试将其转换为相对于给定目录的绝对路径
    ///
    /// ### 参数
    /// * `dir` - 基准目录，用于将相对路径转换为绝对路径
    ///
    /// ### 返回值
    /// 返回包含两个布尔值的元组 `(path1存在, path2存在)`
    pub fn if_exist(&mut self, dir: &Path) -> (bool, bool) {
        let make_absolute = |path: &mut PathBuf| {
            if path.is_relative() {
                *path = dir.join(path.file_name().unwrap_or(OsStr::new("")));
            }
        };

        make_absolute(&mut self.path1);
        make_absolute(&mut self.path2);

        (self.path1.exists(), self.path2.exists())
    }

    /// 判断路径是文件还是目录
    ///
    /// ### 返回值
    /// 返回包含两个布尔值的元组 `(path1是文件, path2是文件)`
    /// * `true` - 路径指向文件
    /// * `false` - 路径指向目录
    pub fn if_file(&self) -> (bool, bool) {
        (self.path1.is_file(), self.path2.is_file())
    }

    /// 检查两个路径是否位于同一个父目录
    ///
    /// 这个方法用于判断两个路径是否在同一个文件夹中，这对于确定重命名操作的安全性很重要。
    /// 如果两个路径在同一目录，某些重命名操作可能不需要临时文件。
    ///
    /// ### 返回值
    /// * `true` - 两个路径在同一个父目录
    /// * `false` - 两个路径在不同的父目录
    pub fn if_same_dir(&self) -> bool {
        self.path1.parent().unwrap() == self.path2.parent().unwrap()
    }

    /// 检测两个路径之间是否存在包含关系（父子目录问题）
    ///
    /// 这个方法用于判断两个路径之间是否有包含关系，这对于确定重命名顺序至关重要。
    /// 当两个目录有父子关系时，重命名顺序会直接影响操作的成功与否。
    ///
    /// ### 返回值
    /// * `1` - path1 包含 path2（path1 是 path2 的父目录或祖先目录）
    /// * `2` - path2 包含 path1（path2 是 path1 的父目录或祖先目录）
    /// * `0` - 不存在包含关系
    pub fn if_root(&self) -> u8 {
        if Self::path_is_parent(&self.path1, &self.path2) {
            1 // path1 包含 path2
        } else if Self::path_is_parent(&self.path2, &self.path1) {
            2 // path2 包含 path1
        } else {
            0 // 不存在包含关系
        }
    }

    /// 辅助函数：检查是否是父子目录关系
    ///
    /// 判断 potential_parent 是否是 potential_child 的父目录或祖先目录
    ///
    /// ### 参数
    /// * `potential_parent` - 可能的父目录路径
    /// * `potential_child` - 可能的子目录路径
    ///
    /// ### 返回值
    /// * `true` - 确实存在父子关系
    /// * `false` - 不存在父子关系
    fn path_is_parent(potential_parent: &Path, potential_child: &Path) -> bool {
        // 尝试确定 child 相对于 parent 的路径
        match potential_child.strip_prefix(potential_parent) {
            Ok(_) => true,   // 如果成功，说明是父子关系
            Err(_) => false, // 如果失败，说明不是父子关系
        }
    }

    /// 获取文件或目录的元数据信息
    ///
    /// 提取路径的文件名（无后缀）、扩展名和父目录路径
    ///
    /// ### 参数
    /// * `file_path` - 要处理的文件或目录路径
    /// * `is_file` - 指示路径是文件还是目录
    ///
    /// ### 返回值
    /// 返回包含元数据的 `MetadataCollection` 结构体
    fn get_info(file_path: &Path, is_file: bool) -> MetadataCollection {
        // 提取字符串的闭包函数，处理文件名和扩展名
        // 如果处理扩展名，会添加前导点"."
        let get_string_closure = |original_result: &Option<&OsStr>, is_ext: bool| {
            match original_result {
                Some(i) => {
                    if is_ext {
                        // 是否在计算后缀，如果是，添加前导点"."
                        ".".to_owned() + i.to_str().unwrap()
                    } else {
                        i.to_str().unwrap().to_string()
                    }
                }
                /*
                取不到就无视
                因前面已经核验完毕，所以此处如果出现Err则是特殊文件命名所致，不影响后面所有操作。
                e.g. "C:\\.cargo\\.config"，该文件取不到后缀，该文件夹也取不到后缀
                */
                None => String::new(),
            }
        };

        if !is_file {
            // 处理目录路径
            MetadataCollection {
                name: {
                    // 对于目录，名称包括主干和扩展名（如果有）
                    get_string_closure(&file_path.file_stem(), false)
                        + get_string_closure(&file_path.extension(), true).as_ref()
                },
                ext: String::new(), // 目录没有扩展名
                parent_dir: {
                    match &file_path.parent() {
                        Some(i) => i.to_path_buf(),
                        None => PathBuf::new(),
                    }
                },
            }
        } else {
            // 处理文件路径
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

    /// 收集两个路径的元数据信息
    ///
    /// ### 参数
    /// * `is_file1` - 指示 path1 是文件还是目录
    /// * `is_file2` - 指示 path2 是文件还是目录
    ///
    /// ### 返回值
    /// 返回包含两个元数据集合的元组 `(path1元数据, path2元数据)`
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

impl Default for GetPathInfo {
    /// 创建包含空路径的默认实例
    fn default() -> Self {
        Self {
            path1: PathBuf::new(),
            path2: PathBuf::new(),
        }
    }
}
