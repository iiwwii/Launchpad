// 后端 Tauri command 封装。前端用 camelCase 参数名，Tauri 自动映射到 Rust snake_case。
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import type { AppConfig, Entry } from "../types";

export const listEntries = (dir: string) =>
  invoke<Entry[]>("list_entries", { dir });

export const launchItem = (path: string) =>
  invoke<void>("launch_item", { path });

export const createShortcut = (target: string, destDir: string, name?: string) =>
  invoke<string>("create_shortcut", { target, destDir, name });

export const setItemMeta = (
  dir: string,
  fileName: string,
  name?: string,
  order?: number,
) => invoke<void>("set_item_meta", { dir, fileName, name, order });

export const revealInExplorer = (path: string) =>
  invoke<void>("reveal_in_explorer", { path });

export const deleteShortcut = (path: string) =>
  invoke<void>("delete_shortcut", { path });

export const getConfig = () => invoke<AppConfig>("get_config");
export const setConfig = (config: AppConfig) =>
  invoke<void>("set_config", { config });

/** 重新注册全局快捷键；传 hotkey 用指定值，否则用已保存配置 */
export const rebindHotkey = (hotkey?: string) =>
  invoke<void>("rebind_hotkey", hotkey ? { hotkey } : {});

/** 临时暂停全局快捷键（录制新快捷键时用） */
export const pauseHotkey = () => invoke<void>("pause_hotkey");

/** 实时调节一级界面 acrylic 强度 */
export const setMainBlur = (blur: number) =>
  invoke<void>("set_main_blur", { blur });

/** 后端返回的图标绝对路径 → webview 可访问的 asset URL。 */
export const iconUrl = (path: string | null) =>
  path ? convertFileSrc(path) : null;
