import { useCallback, useEffect, useState } from "react";
import type { MouseEvent } from "react";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import type { DragEndEvent } from "@dnd-kit/core";
import { SortableContext, arrayMove, rectSortingStrategy } from "@dnd-kit/sortable";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import {
  createShortcut,
  deleteShortcut,
  getConfig,
  launchItem,
  listEntries,
  revealInExplorer,
  setItemMeta,
} from "../lib/api";
import { useStore } from "../store/useStore";
import type { Entry } from "../types";
import ContextMenu from "./ContextMenu";
import EntryCard from "./EntryCard";
import FolderPopup from "./FolderPopup";
import FolderPreview from "./FolderPreview";
import SearchBar from "./SearchBar";
import SettingsPanel from "./SettingsPanel";

export default function LaunchpadView() {
  const {
    config,
    setConfig,
    currentDir,
    setDir,
    entries,
    setEntries,
    loading,
    setLoading,
    search,
    setSearch,
    searchResults,
    setSearchResults,
    openedFolder,
    setOpenedFolder,
    folderClosing,
    setFolderClosing,
  } = useStore();

  const [settingsOpen, setSettingsOpen] = useState(false);
  const [ctxMenu, setCtxMenu] = useState<{
    entry: Entry;
    x: number;
    y: number;
  } | null>(null);
  const [activeId, setActiveId] = useState<string | null>(null);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
  );

  // 主题 → CSS 变量
  useEffect(() => {
    if (!config) return;
    const root = document.documentElement;
    root.style.setProperty("--columns", String(config.columns));
    root.style.setProperty("--accent", config.accent_color);
    root.style.setProperty("--main-blur", `${config.main_blur}px`);
    root.style.setProperty("--folder-blur", `${config.folder_blur}px`);
    root.style.setProperty(
      "--overlay-alpha",
      String(0.12 + (config.folder_blur / 80) * 0.5),
    );
    root.style.setProperty(
      "--text-color",
      config.main_blur <= 30 ? "#1a1a1a" : "#fff",
    );
    root.style.setProperty("color-scheme", config.theme);
  }, [config]);

  useEffect(() => {
    void (async () => {
      try {
        const cfg = await getConfig();
        setConfig(cfg);
        setDir(cfg.root_path, true);
      } catch (e) {
        console.error("加载配置失败", e);
      }
    })();
  }, [setConfig, setDir]);

  const loadDir = useCallback(
    async (dir: string) => {
      if (!dir) return;
      setLoading(true);
      try {
        setEntries(await listEntries(dir));
      } catch (e) {
        console.error("读取目录失败", e);
        setEntries([]);
      } finally {
        setLoading(false);
      }
    },
    [setEntries, setLoading],
  );

  useEffect(() => {
    if (currentDir) void loadDir(currentDir);
  }, [currentDir, loadDir]);

  // 搜索：递归根 + 所有分类
  useEffect(() => {
    const q = search.trim().toLowerCase();
    if (!q || !config) {
      setSearchResults(null);
      return;
    }
    let cancelled = false;
    void (async () => {
      const out: Entry[] = [];
      try {
        const rootEntries = await listEntries(config.root_path);
        for (const e of rootEntries) {
          if (e.name.toLowerCase().includes(q)) out.push(e);
          if (e.kind === "folder") {
            try {
              const sub = await listEntries(e.path);
              for (const s of sub) {
                if (s.kind === "file" && s.name.toLowerCase().includes(q))
                  out.push(s);
              }
            } catch {
              /* 忽略 */
            }
          }
        }
      } catch (e) {
        console.error("搜索失败", e);
      }
      if (!cancelled) setSearchResults(out);
    })();
    return () => {
      cancelled = true;
    };
  }, [search, config, setSearchResults]);

  // 外部文件拖入 → 建 .lnk
  useEffect(() => {
    const win = getCurrentWindow();
    const unlisten = win.onDragDropEvent((event) => {
      if (event.payload.type === "drop") {
        const dir = useStore.getState().currentDir;
        void Promise.all(event.payload.paths.map((p) => createShortcut(p, dir))).then(
          () => loadDir(dir),
        );
      }
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [loadDir]);

  // 文件监听 → 自动刷新
  useEffect(() => {
    const unlisten = listen("launchpad:changed", () => {
      void loadDir(useStore.getState().currentDir);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [loadDir]);

  // 聚焦=呼出（回到干净新主页）；失焦=收起
  useEffect(() => {
    const win = getCurrentWindow();
    let wasFocused = false;
    const unlisten = win.onFocusChanged(({ payload: focused }) => {
      if (focused) {
        wasFocused = true;
        setSearch("");
        setOpenedFolder(null);
      } else if (wasFocused) {
        void win.hide();
      }
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [setSearch, setOpenedFolder]);

  const handleOpen = useCallback(
    async (entry: Entry) => {
      if (entry.kind === "folder") {
        setOpenedFolder(entry); // 弹出文件夹框（Mac 风格）
        setSearch("");
      } else {
        // 立即反馈：先收起窗口，程序后台异步启动（不阻塞 UI，避免点击后卡顿）
        setOpenedFolder(null);
        if (config?.hide_after_launch) void getCurrentWindow().hide();
        void launchItem(entry.path).catch((e) => console.error("启动失败", e));
      }
    },
    [config, setOpenedFolder, setSearch],
  );

  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      const { active, over } = event;
      if (!over || active.id === over.id) return;
      const oldIndex = entries.findIndex((e) => e.path === active.id);
      const newIndex = entries.findIndex((e) => e.path === over.id);
      if (oldIndex < 0 || newIndex < 0) return;
      const reordered = arrayMove(entries, oldIndex, newIndex);
      setEntries(reordered);
      const dir = useStore.getState().currentDir;
      void Promise.all(
        reordered.map((e, i) => setItemMeta(dir, e.file_name, undefined, i)),
      ).then(() => loadDir(dir));
    },
    [entries, loadDir, setEntries],
  );

  const handleRename = useCallback(
    async (entry: Entry) => {
      const newName = window.prompt("重命名", entry.name);
      if (newName && newName.trim() && newName !== entry.name) {
        const dir = useStore.getState().currentDir;
        await setItemMeta(dir, entry.file_name, newName.trim());
        void loadDir(dir);
      }
    },
    [loadDir],
  );

  const handleDelete = useCallback(
    async (entry: Entry) => {
      if (
        !window.confirm(
          `删除快捷方式「${entry.name}」？\n（仅删除快捷方式，不影响原程序）`,
        )
      )
        return;
      try {
        await deleteShortcut(entry.path);
        void loadDir(useStore.getState().currentDir);
      } catch (e) {
        console.error("删除失败", e);
      }
    },
    [loadDir],
  );

  const handleReveal = useCallback(async (entry: Entry) => {
    try {
      await revealInExplorer(entry.path);
    } catch (e) {
      console.error("打开文件夹失败", e);
    }
  }, []);

  // 点空白：文件夹框开着→关框；否则收起启动台
  const onSpaceDown = (e: MouseEvent) => {
    if (e.target !== e.currentTarget) return;
    if (openedFolder) setOpenedFolder(null);
    else void getCurrentWindow().hide();
  };

  const searching = search.trim().length > 0;
  const shown = searching ? searchResults ?? [] : entries;
  const activeEntry = activeId
    ? shown.find((e) => e.path === activeId)
    : undefined;

  return (
    <>
    <div
      className={`backdrop${openedFolder && !folderClosing ? " dimmed" : ""}`}
      onMouseDown={onSpaceDown}
    >
      <div className="lp" onMouseDown={onSpaceDown}>
        <SearchBar value={search} onChange={setSearch} />

        <button
          className="icon-btn gear"
          title="设置"
          onMouseDown={(e) => e.stopPropagation()}
          onClick={() => config && setSettingsOpen(true)}
        >
          ⚙
        </button>

        <div className="grid" onMouseDown={onSpaceDown}>
          <DndContext
            sensors={sensors}
            collisionDetection={closestCenter}
            onDragStart={(e) => setActiveId(String(e.active.id))}
            onDragEnd={(e) => {
              setActiveId(null);
              handleDragEnd(e);
            }}
            onDragCancel={() => setActiveId(null)}
          >
            <SortableContext
              items={shown.map((e) => e.path)}
              strategy={rectSortingStrategy}
            >
              {shown.map((e) => (
                <EntryCard
                  key={e.path}
                  entry={e}
                  onOpen={handleOpen}
                  onContextMenu={(entry, x, y) => setCtxMenu({ entry, x, y })}
                  disabled={searching}
                />
              ))}
            </SortableContext>
            <DragOverlay dropAnimation={null}>
              {activeEntry ? <DragPreview entry={activeEntry} /> : null}
            </DragOverlay>
          </DndContext>
          {shown.length === 0 && !loading && (
            <div className="empty">
              {searching
                ? "无匹配结果"
                : "这里还没有东西，往文件夹里放些快捷方式吧"}
            </div>
          )}
        </div>
      </div>
    </div>

      {openedFolder && (
        <FolderPopup
          folder={openedFolder}
          onClosing={() => setFolderClosing(true)}
          onClose={() => {
            setOpenedFolder(null);
            setFolderClosing(false);
          }}
          onOpen={handleOpen}
        />
      )}

      {ctxMenu && (
        <ContextMenu
          x={ctxMenu.x}
          y={ctxMenu.y}
          canDelete={ctxMenu.entry.kind === "file"}
          onOpen={() => void handleOpen(ctxMenu.entry)}
          onRename={() => void handleRename(ctxMenu.entry)}
          onReveal={() => void handleReveal(ctxMenu.entry)}
          onDelete={() => void handleDelete(ctxMenu.entry)}
          onClose={() => setCtxMenu(null)}
        />
      )}

      {settingsOpen && config && (
        <SettingsPanel
          config={config}
          onClose={() => setSettingsOpen(false)}
          onChanged={(c) => {
            setConfig(c);
            void loadDir(useStore.getState().currentDir);
          }}
        />
      )}
    </>
  );
}

/** 拖动时跟随鼠标的预览卡片（非 sortable，避免冲突） */
function DragPreview({ entry }: { entry: Entry }) {
  const icon = entry.icon ? convertFileSrc(entry.icon) : null;
  return (
    <div className="card drag-preview">
      <div className="card-icon">
        {entry.kind === "folder" ? (
          <FolderPreview path={entry.path} />
        ) : icon ? (
          <img src={icon} alt="" draggable={false} />
        ) : (
          <span className="card-fallback">📄</span>
        )}
      </div>
      <div className="card-name">{entry.name}</div>
    </div>
  );
}
