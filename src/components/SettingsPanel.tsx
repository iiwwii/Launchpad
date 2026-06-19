import { useEffect, useState } from "react";
import type { AppConfig } from "../types";
import { pauseHotkey, rebindHotkey, setConfig, setMainBlur } from "../lib/api";

interface Props {
  config: AppConfig;
  onClose: () => void;
  onChanged: (c: AppConfig) => void;
}

/** 键盘事件 → 快捷键组成（与后端 parse_hotkey 对齐）。 */
function mapKey(e: KeyboardEvent): string | null {
  const c = e.code;
  if (c.startsWith("Key")) return c.slice(3);
  if (c.startsWith("Digit")) return c.slice(5);
  if (/^F\d+$/.test(c)) return c;
  switch (c) {
    case "Space":
      return "Space";
    case "Enter":
      return "Enter";
    case "Tab":
      return "Tab";
    case "Backspace":
      return "Backspace";
    case "ArrowUp":
      return "Up";
    case "ArrowDown":
      return "Down";
    case "ArrowLeft":
      return "Left";
    case "ArrowRight":
      return "Right";
    case "Semicolon":
      return ";";
    case "Comma":
      return ",";
    case "Period":
      return ".";
    case "Slash":
      return "/";
    case "Backquote":
      return "`";
    case "Minus":
      return "-";
    case "Equal":
      return "=";
    case "Backslash":
      return "\\";
    case "Quote":
      return "'";
  }
  return null;
}

/** 点击进入录制，按下组合键即捕获。录制时临时暂停全局快捷键，避免系统干扰。 */
function HotkeyField({
  value,
  onChange,
}: {
  value: string;
  onChange: (v: string) => void;
}) {
  const [recording, setRecording] = useState(false);

  // 开始录制 → 暂停全局快捷键
  useEffect(() => {
    if (recording) void pauseHotkey();
  }, [recording]);

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      if (e.key === "Escape") {
        setRecording(false);
        void rebindHotkey(); // 恢复
        return;
      }
      const parts: string[] = [];
      if (e.ctrlKey) parts.push("Ctrl");
      if (e.altKey) parts.push("Alt");
      if (e.shiftKey) parts.push("Shift");
      if (e.metaKey) parts.push("Super");
      const k = mapKey(e);
      if (k) {
        parts.push(k);
        if (parts.length > 1) {
          const hk = parts.join("+");
          onChange(hk);
          setRecording(false);
          void rebindHotkey(hk); // 即时用新快捷键
        }
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, onChange]);

  return (
    <button
      type="button"
      className="hotkey-field"
      onClick={() => setRecording((r) => !r)}
    >
      {recording ? "按下组合键…（Esc 取消）" : value || "点击设置"}
    </button>
  );
}

export default function SettingsPanel({ config, onClose, onChanged }: Props) {
  const [draft, setDraft] = useState<AppConfig>(config);

  const update = (patch: Partial<AppConfig>) =>
    setDraft((d) => ({ ...d, ...patch }));

  // 关闭时恢复全局快捷键（避免录制中暂停后关了设置，没法呼出）
  const close = () => {
    void rebindHotkey();
    onClose();
  };

  const save = async () => {
    try {
      await setConfig(draft);
      onChanged(draft);
      close(); // close 内 rebind（此时 config 已是新）
    } catch (e) {
      console.error("保存配置失败", e);
    }
  };

  return (
    <div
      className="settings-overlay"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) close();
      }}
    >
      <div className="settings-panel" onMouseDown={(e) => e.stopPropagation()}>
        <h2>设置</h2>

        <label className="field">
          <span>根文件夹路径</span>
          <input
            value={draft.root_path}
            onChange={(e) => update({ root_path: e.target.value })}
          />
        </label>

        <label className="field">
          <span>呼出快捷键</span>
          <HotkeyField
            value={draft.hotkey}
            onChange={(v) => update({ hotkey: v })}
          />
        </label>

        <label className="field">
          <span>主题</span>
          <select
            value={draft.theme}
            onChange={(e) => update({ theme: e.target.value as AppConfig["theme"] })}
          >
            <option value="auto">跟随系统</option>
            <option value="dark">深色</option>
            <option value="light">浅色</option>
          </select>
        </label>

        <label className="field">
          <span>强调色</span>
          <input
            type="color"
            value={draft.accent_color}
            onChange={(e) => update({ accent_color: e.target.value })}
          />
        </label>

        <label className="field">
          <span>一级界面背景（{draft.main_blur > 30 ? "深色" : "浅色"} mica）</span>
          <input
            type="range"
            min={0}
            max={60}
            value={draft.main_blur}
            onChange={(e) => {
              const v = Number(e.target.value);
              update({ main_blur: v });
              void setMainBlur(v);
              document.documentElement.style.setProperty(
                "--text-color",
                v <= 30 ? "#1a1a1a" : "#fff",
              );
            }}
          />
        </label>

        <label className="field">
          <span>二级界面毛玻璃（{draft.folder_blur}px）</span>
          <input
            type="range"
            min={0}
            max={80}
            value={draft.folder_blur}
            onChange={(e) => {
              const v = Number(e.target.value);
              update({ folder_blur: v });
              const root = document.documentElement;
              root.style.setProperty("--folder-blur", `${v}px`);
              root.style.setProperty(
                "--overlay-alpha",
                String(0.12 + (v / 80) * 0.5),
              );
            }}
          />
        </label>

        <label className="field">
          <span>每行列数（{draft.columns}）</span>
          <input
            type="range"
            min={4}
            max={12}
            value={draft.columns}
            onChange={(e) => update({ columns: Number(e.target.value) })}
          />
        </label>

        <label className="field checkbox">
          <input
            type="checkbox"
            checked={draft.hide_after_launch}
            onChange={(e) => update({ hide_after_launch: e.target.checked })}
          />
          <span>启动后自动收起</span>
        </label>

        <label className="field checkbox">
          <input
            type="checkbox"
            checked={draft.auto_start}
            onChange={(e) => update({ auto_start: e.target.checked })}
          />
          <span>开机自启</span>
        </label>

        <div className="settings-actions">
          <button className="ghost" onClick={close}>
            取消
          </button>
          <button className="primary" onClick={save}>
            保存
          </button>
        </div>
      </div>
    </div>
  );
}
