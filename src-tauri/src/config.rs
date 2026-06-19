// 全局应用配置，存放在 Tauri app config dir（不污染用户的根文件夹）。
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_root")]
    pub root_path: String,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    #[serde(default = "default_theme")]
    pub theme: String, // auto | light | dark
    #[serde(default = "default_accent")]
    pub accent_color: String,
    #[serde(default = "default_main_blur")]
    pub main_blur: u32, // 一级界面（首页）毛玻璃强度
    #[serde(default = "default_folder_blur")]
    pub folder_blur: u32, // 二级界面（文件夹框）毛玻璃强度
    #[serde(default = "default_columns")]
    pub columns: u32,
    #[serde(default = "default_true")]
    pub hide_after_launch: bool,
    #[serde(default = "default_true")]
    pub auto_start: bool,
}

fn default_root() -> String {
    desktop_dir()
        .map(|d| d.join("启动台").to_string_lossy().to_string())
        .unwrap_or_else(|| r"D:\Ticea\启动台".to_string())
}

/// 获取系统桌面目录（Windows 用 SHGetKnownFolderPath，避免引入 dirs 触发网络下载）。
#[cfg(windows)]
fn desktop_dir() -> Option<std::path::PathBuf> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{KF_FLAG_DEFAULT, SHGetKnownFolderPath, FOLDERID_Desktop};
    unsafe {
        let pwstr = SHGetKnownFolderPath(&FOLDERID_Desktop, KF_FLAG_DEFAULT, None).ok()?;
        let s = PCWSTR(pwstr.0 as *const u16).to_string().ok()?;
        CoTaskMemFree(Some(pwstr.0 as *const _));
        Some(std::path::PathBuf::from(s))
    }
}

#[cfg(not(windows))]
fn desktop_dir() -> Option<std::path::PathBuf> {
    None
}
fn default_hotkey() -> String {
    "Ctrl+Alt+Space".to_string()
}
fn default_theme() -> String {
    "auto".to_string()
}
fn default_accent() -> String {
    "#007aff".to_string()
}
fn default_main_blur() -> u32 {
    30
}
fn default_folder_blur() -> u32 {
    50
}
fn default_columns() -> u32 {
    7
}
fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            root_path: default_root(),
            hotkey: default_hotkey(),
            theme: default_theme(),
            accent_color: default_accent(),
            main_blur: default_main_blur(),
            folder_blur: default_folder_blur(),
            columns: default_columns(),
            hide_after_launch: true,
            auto_start: true,
        }
    }
}

impl AppConfig {
    pub fn config_file(dir: &Path) -> PathBuf {
        dir.join("config.json")
    }

    pub fn load(dir: &Path) -> Self {
        fs::read_to_string(Self::config_file(dir))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        let path = Self::config_file(dir);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let s = serde_json::to_string_pretty(self).unwrap_or_default();
        fs::write(path, s)
    }
}
