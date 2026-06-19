// Tauri 命令：列目录、启动、建快捷方式、改元数据、配置、资源管理器定位。
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::Manager;

use crate::config::AppConfig;
use crate::launch;
use crate::lnk;
use crate::metadata::{FolderMeta, ItemMeta, META_FILE};

/// 一个条目（启动项文件 或 分类文件夹）。
#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub name: String,
    pub file_name: String,
    pub path: String,
    pub kind: String, // "file" | "folder"
    pub icon: Option<String>,
    pub is_lnk: bool,
    pub is_url: bool,
}

fn cache_dir_of(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_cache_dir()
        .map_err(|e| e.to_string())
}

fn config_dir_of(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|e| e.to_string())
}

fn ext_lower(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default()
}

/// 列出某层（根或某分类文件夹）的条目，合并 .launchpad.json 的排序/改名，附带图标路径。
#[tauri::command]
pub fn list_entries(dir: String, app: tauri::AppHandle) -> Result<Vec<Entry>, String> {
    let cache_dir = cache_dir_of(&app)?;
    let path = Path::new(&dir);
    let meta = FolderMeta::load(path);

    let mut entries: Vec<Entry> = Vec::new();
    let read = std::fs::read_dir(path).map_err(|e| e.to_string())?;
    for entry in read.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name == META_FILE || file_name.starts_with('.') {
            continue;
        }
        let ft = entry.file_type().map_err(|e| e.to_string())?;
        let is_dir = ft.is_dir();
        let file_path = dunce::canonicalize(entry.path()).unwrap_or_else(|_| entry.path());
        let el = ext_lower(&file_path);

        let display_name = meta
            .items
            .get(&file_name)
            .and_then(|m| m.name.clone())
            .unwrap_or_else(|| {
                if is_dir {
                    file_name.clone()
                } else {
                    file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&file_name)
                        .to_string()
                }
            });

        let icon = crate::icon::get_icon_url(&file_path.to_string_lossy(), &cache_dir)
            .map(|p| p.to_string_lossy().to_string());

        entries.push(Entry {
            name: display_name,
            file_name,
            path: file_path.to_string_lossy().to_string(),
            kind: if is_dir { "folder".into() } else { "file".into() },
            icon,
            is_lnk: el == "lnk",
            is_url: el == "url",
        });
    }

    // 排序：有自定义 order 的在前（按 order），其余按文件名在后。
    entries.sort_by(|a, b| {
        let oa = meta.items.get(&a.file_name).and_then(|m| m.order);
        let ob = meta.items.get(&b.file_name).and_then(|m| m.order);
        match (oa, ob) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.file_name.cmp(&b.file_name),
        }
    });

    Ok(entries)
}

/// 启动/打开（ShellExecuteW 会自动解引用 .lnk）。
#[tauri::command]
pub fn launch_item(path: String) -> Result<(), String> {
    launch::launch(&path).map_err(|e| e.to_string())
}

/// 在 dest_dir 创建指向 target 的 .lnk，返回生成的 .lnk 路径。
#[tauri::command]
pub fn create_shortcut(
    target: String,
    dest_dir: String,
    name: Option<String>,
) -> Result<String, String> {
    let target_path = Path::new(&target);
    let base = name.unwrap_or_else(|| {
        target_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("shortcut")
            .to_string()
    });
    let lnk_path = unique_lnk_path(Path::new(&dest_dir), &base);
    lnk::create_lnk(
        &target,
        &lnk_path.to_string_lossy(),
        None,
        None,
        None,
    )
    .map_err(|e| e.to_string())?;
    Ok(lnk_path.to_string_lossy().to_string())
}

/// 写入某条目的自定义显示名/顺序到该文件夹的 .launchpad.json。
#[tauri::command]
pub fn set_item_meta(
    dir: String,
    file_name: String,
    name: Option<String>,
    order: Option<i32>,
) -> Result<(), String> {
    let mut meta = FolderMeta::load(Path::new(&dir));
    let item: &mut ItemMeta = meta.items.entry(file_name).or_default();
    if let Some(n) = name {
        item.name = Some(n);
    }
    if let Some(o) = order {
        item.order = Some(o);
    }
    meta.save(Path::new(&dir)).map_err(|e| e.to_string())
}

/// 在资源管理器中定位到该文件/文件夹。
#[tauri::command]
pub fn reveal_in_explorer(path: String) -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg(format!("/select,{}", path))
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 删除一个快捷方式文件（仅允许 .lnk，二次确认由前端做）。
#[tauri::command]
pub fn delete_shortcut(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if ext_lower(p) != "lnk" {
        return Err("只允许删除快捷方式(.lnk)".into());
    }
    std::fs::remove_file(p).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_config(app: tauri::AppHandle) -> Result<AppConfig, String> {
    let dir = config_dir_of(&app)?;
    Ok(AppConfig::load(&dir))
}

#[tauri::command]
pub fn set_config(config: AppConfig, app: tauri::AppHandle) -> Result<(), String> {
    let dir = config_dir_of(&app)?;
    config.save(&dir).map_err(|e| e.to_string())
}

fn unique_lnk_path(dest_dir: &Path, base: &str) -> PathBuf {
    let direct = dest_dir.join(format!("{}.lnk", base));
    if !direct.exists() {
        return direct;
    }
    for i in 1..1000 {
        let p = dest_dir.join(format!("{} ({}).lnk", base, i));
        if !p.exists() {
            return p;
        }
    }
    direct
}
