// 与后端 Rust 结构体字段对齐（serde 默认 snake_case）。
export interface Entry {
  name: string;
  file_name: string;
  path: string;
  kind: "file" | "folder";
  icon: string | null;
  is_lnk: boolean;
  is_url: boolean;
}

export type Theme = "auto" | "light" | "dark";

export interface AppConfig {
  root_path: string;
  hotkey: string;
  theme: Theme;
  accent_color: string;
  main_blur: number;
  folder_blur: number;
  columns: number;
  hide_after_launch: boolean;
  auto_start: boolean;
}
