import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { emit } from "@tauri-apps/api/event";
import LaunchpadView from "./components/LaunchpadView";

export default function App() {
  // ESC 收起（启动台用 hide 便于唤回）
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") void getCurrentWindow().hide();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // 首次渲染完通知后端 show，避免 show 时未渲染 + acrylic 过渡
  useEffect(() => {
    void emit("launchpad:ready");
  }, []);

  return <LaunchpadView />;
}
