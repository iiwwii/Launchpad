// 每个文件夹的元数据：自定义显示名、排序、（可选）图标/颜色。
// 存为该文件夹下的隐藏文件 .launchpad.json，跟着文件夹走、可移植。
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub const META_FILE: &str = ".launchpad.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FolderMeta {
    /// key = 该层的条目文件名（不是显示名）
    #[serde(default)]
    pub items: HashMap<String, ItemMeta>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ItemMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

impl FolderMeta {
    pub fn load(dir: &Path) -> Self {
        fs::read_to_string(dir.join(META_FILE))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        let path = dir.join(META_FILE);
        let s = serde_json::to_string_pretty(self).unwrap_or_default();
        fs::write(&path, s)?;
        hide_on_windows(&path);
        Ok(())
    }
}

/// Windows 下给文件设隐藏属性，资源管理器默认不显示（避免 .launchpad.json 碍眼）。
#[cfg(windows)]
fn hide_on_windows(path: &Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN};
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    unsafe {
        let _ = SetFileAttributesW(PCWSTR(wide.as_ptr()), FILE_ATTRIBUTE_HIDDEN);
    }
}

#[cfg(not(windows))]
fn hide_on_windows(_path: &Path) {}
