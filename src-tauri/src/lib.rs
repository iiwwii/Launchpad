// 启动台后端入口：全屏毛玻璃窗口 + 全局快捷键呼出 + 开机自启 + 文件监听刷新。
#![allow(dead_code)]

use std::path::Path;
use std::time::{Duration, Instant};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Listener, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

mod commands;
mod config;
mod icon;
mod launch;
mod lnk;
mod metadata;

use crate::config::AppConfig;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    // 快捷键呼出/收起
                    if event.state == ShortcutState::Pressed {
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .invoke_handler(tauri::generate_handler![
            commands::list_entries,
            commands::launch_item,
            commands::create_shortcut,
            commands::set_item_meta,
            commands::reveal_in_explorer,
            commands::delete_shortcut,
            commands::get_config,
            commands::set_config,
            rebind_hotkey,
            pause_hotkey,
            set_main_blur,
        ])
        .setup(|app| {
            let window = app
                .get_webview_window("main")
                .expect("main window missing");
            // 全屏铺满（避开 fullscreen/maximized 的透明 bug）
            span_all_monitors(&window);

            // 不在任务栏显示图标（呼出/收起时任务栏不会闪动）
            let _ = window.set_skip_taskbar(true);

            // 配置
            let config_dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&config_dir).ok();
            let config = AppConfig::load(&config_dir);

            // 默认根目录（桌面/启动台）如不存在则创建，首次运行可用
            let _ = std::fs::create_dir_all(&config.root_path);

            // Mica 背景（Win11 原生，丝滑无过渡）：明暗由 main_blur 控制（>30 深色）
            let _ = window_vibrancy::apply_mica(&window, mica_dark(config.main_blur));
            // 非置顶：让系统 alt+tab / win+tab 等切换界面能盖在启动台之上
            let _ = window.set_always_on_top(false);

            // 系统托盘（右下角图标，方便管理）
            let show_item = MenuItem::with_id(app, "show", "显示启动台", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &quit_item])?;
            let _ = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().cloned().unwrap())
                .tooltip("启动台")
                .menu(&tray_menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app);

            // 全局快捷键
            if let Some(shortcut) = parse_hotkey(&config.hotkey) {
                if let Err(e) = app.global_shortcut().register(shortcut) {
                    eprintln!("注册快捷键 {:?} 失败: {}", config.hotkey, e);
                }
            }

            // 开机自启（按配置）
            let mgr = app.autolaunch();
            let _ = if config.auto_start {
                mgr.enable()
            } else {
                mgr.disable()
            };

            // 文件监听：根目录变化 → emit
            start_watcher(app.handle().clone(), &config.root_path);

            // 等前端渲染完再 show：避免 show 时 webview 未渲染完叠加 acrylic 过渡，
            // 造成“全屏虚化→桌面虚化”的两段不丝滑感。
            let win_for_show = window.clone();
            app.listen("launchpad:ready".to_string(), move |_| {
                let _ = win_for_show.show();
                let _ = win_for_show.set_focus();
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 计算所有显示器包围盒并把窗口铺满（处理副屏负坐标）。
fn span_all_monitors(window: &tauri::WebviewWindow) {
    let monitors = window.available_monitors().unwrap_or_default();
    let bbox = monitors.iter().fold(
        None::<(i32, i32, i32, i32)>,
        |acc, m| {
            let p = m.position();
            let s = m.size();
            let (x1, y1, x2, y2) = (p.x, p.y, p.x + s.width as i32, p.y + s.height as i32);
            Some(match acc {
                None => (x1, y1, x2, y2),
                Some((ax1, ay1, ax2, ay2)) => {
                    (ax1.min(x1), ay1.min(y1), ax2.max(x2), ay2.max(y2))
                }
            })
        },
    );
    if let Some((min_x, min_y, max_x, max_y)) = bbox {
        let _ = window.set_position(tauri::PhysicalPosition::new(min_x, min_y));
        let _ = window.set_size(tauri::PhysicalSize::new(
            (max_x - min_x) as u32,
            (max_y - min_y) as u32,
        ));
    }
}

/// 重新注册全局快捷键。传 hotkey 用指定值，否则用已保存配置。
#[tauri::command]
fn rebind_hotkey(app: tauri::AppHandle, hotkey: Option<String>) -> Result<(), String> {
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let config = AppConfig::load(&config_dir);
    let hk = hotkey.unwrap_or(config.hotkey);
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    if let Some(sc) = parse_hotkey(&hk) {
        gs.register(sc).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 临时暂停全局快捷键（设置里录制新快捷键时用，避免系统快捷键干扰捕获）。
#[tauri::command]
fn pause_hotkey(app: tauri::AppHandle) -> Result<(), String> {
    let _ = app.global_shortcut().unregister_all();
    Ok(())
}

/// main_blur(0-60) → mica 明暗。>30 深色 mica，否则浅色。
fn mica_dark(blur: u32) -> Option<bool> {
    Some(blur > 30)
}

/// 实时调节一级界面（主窗口）mica 明暗。
#[tauri::command]
fn set_main_blur(window: tauri::WebviewWindow, blur: u32) -> Result<(), String> {
    let _ = window_vibrancy::apply_mica(&window, mica_dark(blur));
    Ok(())
}

/// 启动文件监听：根目录变化 → emit `launchpad:changed`（防抖 300ms）。
fn start_watcher(app: tauri::AppHandle, root: &str) {
    if root.trim().is_empty() {
        return;
    }
    let root = root.to_string();
    std::thread::spawn(move || {
        use notify::{EventKind, Watcher};
        let (tx, rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
        let mut watcher = match notify::recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("启动文件监听失败: {}", e);
                return;
            }
        };
        if let Err(e) = watcher.watch(Path::new(&root), notify::RecursiveMode::Recursive) {
            eprintln!("监听目录失败 {}: {}", root, e);
        }
        let mut last = Instant::now() - Duration::from_secs(60);
        for res in rx {
            if let Ok(ev) = res {
                if matches!(
                    ev.kind,
                    EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_)
                ) && last.elapsed() > Duration::from_millis(300)
                {
                    let _ = app.emit("launchpad:changed", ());
                    last = Instant::now();
                }
            }
        }
    });
}

/// 解析 "Alt+Space" / "Ctrl+Shift+P" 形式的快捷键。
fn parse_hotkey(s: &str) -> Option<Shortcut> {
    let mut modifiers = Modifiers::empty();
    let mut code: Option<Code> = None;
    for part in s.split('+') {
        let p = part.trim().to_ascii_lowercase();
        if p.is_empty() {
            continue;
        }
        match p.as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            "super" | "win" | "meta" | "cmd" | "command" => modifiers |= Modifiers::SUPER,
            _ => match parse_code(&p) {
                Some(c) => code = Some(c),
                None => return None,
            },
        }
    }
    Some(Shortcut::new(Some(modifiers), code?))
}

fn parse_code(s: &str) -> Option<Code> {
    use Code::*;
    let upper = s.to_ascii_uppercase();
    if upper.len() == 1 {
        let c = upper.chars().next()?;
        return match c {
            'A'..='Z' => Some(letter_code(c)),
            '0'..='9' => Some(digit_code(c)),
            _ => None,
        };
    }
    Some(match s {
        "space" => Space,
        "enter" | "return" => Enter,
        "esc" | "escape" => Escape,
        "tab" => Tab,
        "backspace" => Backspace,
        "up" => ArrowUp,
        "down" => ArrowDown,
        "left" => ArrowLeft,
        "right" => ArrowRight,
        _ => {
            if upper.starts_with('F') {
                if let Ok(n) = upper[1..].parse::<u32>() {
                    return f_key(n);
                }
            }
            return None;
        }
    })
}

fn letter_code(c: char) -> Code {
    use Code::*;
    match c {
        'A' => KeyA, 'B' => KeyB, 'C' => KeyC, 'D' => KeyD, 'E' => KeyE, 'F' => KeyF,
        'G' => KeyG, 'H' => KeyH, 'I' => KeyI, 'J' => KeyJ, 'K' => KeyK, 'L' => KeyL,
        'M' => KeyM, 'N' => KeyN, 'O' => KeyO, 'P' => KeyP, 'Q' => KeyQ, 'R' => KeyR,
        'S' => KeyS, 'T' => KeyT, 'U' => KeyU, 'V' => KeyV, 'W' => KeyW, 'X' => KeyX,
        'Y' => KeyY, 'Z' => KeyZ, _ => Space,
    }
}

fn digit_code(c: char) -> Code {
    use Code::*;
    match c {
        '0' => Digit0, '1' => Digit1, '2' => Digit2, '3' => Digit3, '4' => Digit4,
        '5' => Digit5, '6' => Digit6, '7' => Digit7, '8' => Digit8, '9' => Digit9,
        _ => Digit0,
    }
}

fn f_key(n: u32) -> Option<Code> {
    use Code::*;
    Some(match n {
        1 => F1, 2 => F2, 3 => F3, 4 => F4, 5 => F5, 6 => F6, 7 => F7, 8 => F8,
        9 => F9, 10 => F10, 11 => F11, 12 => F12, 13 => F13, 14 => F14, 15 => F15,
        16 => F16, 17 => F17, 18 => F18, 19 => F19, 20 => F20, 21 => F21, 22 => F22,
        23 => F23, 24 => F24, _ => return None,
    })
}
