import { useEffect, useRef, useState, type WheelEvent } from "react";
import {
  DndContext,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import type { DragEndEvent } from "@dnd-kit/core";
import {
  SortableContext,
  arrayMove,
  rectSortingStrategy,
  useSortable,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listEntries, setItemMeta } from "../lib/api";
import type { Entry } from "../types";

const COLS = 4;
const ROWS = 3;
const PAGE_SIZE = COLS * ROWS;

interface Props {
  folder: Entry;
  onClose: () => void;
  onOpen: (e: Entry) => void;
  onClosing: () => void;
}

function SortableItem({
  entry,
  onOpen,
}: {
  entry: Entry;
  onOpen: (e: Entry) => void;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: entry.path });
  const icon = entry.icon ? convertFileSrc(entry.icon) : null;
  return (
    <div
      ref={setNodeRef}
      className={`card-slot${isDragging ? " dragging" : ""}`}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      {...attributes}
      {...listeners}
    >
      <button
        className="card folder-item"
        onClick={() => onOpen(entry)}
        title={entry.name}
      >
        <div className="card-icon">
          {icon ? (
            <img src={icon} alt="" draggable={false} />
          ) : (
            <span className="card-fallback">📄</span>
          )}
        </div>
        <div className="card-name">{entry.name}</div>
      </button>
    </div>
  );
}

/** 文件夹浮动框：4×3 翻页 + 滑动 + 滚轮 + 关闭动画 + 图标拖拽排序。 */
export default function FolderPopup({ folder, onClose, onOpen, onClosing }: Props) {
  const [items, setItems] = useState<Entry[]>([]);
  const [page, setPage] = useState(0);
  const [closing, setClosing] = useState(false);
  const [dir, setDir] = useState<"left" | "right">("left");
  const lastWheel = useRef(0);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
  );

  useEffect(() => {
    let cancelled = false;
    setPage(0);
    setClosing(false);
    listEntries(folder.path)
      .then((list) => {
        if (!cancelled) setItems(list);
      })
      .catch(() => setItems([]));
    return () => {
      cancelled = true;
    };
  }, [folder.path]);

  const pages = Math.max(1, Math.ceil(items.length / PAGE_SIZE));
  const cur = Math.min(page, pages - 1);
  const pageItems = items.slice(cur * PAGE_SIZE, cur * PAGE_SIZE + PAGE_SIZE);
  const cells: (Entry | null)[] = [...pageItems];
  while (cells.length < PAGE_SIZE) cells.push(null);

  const close = () => {
    if (closing) return;
    setClosing(true);
    onClosing();
    window.setTimeout(onClose, 110);
  };

  const go = (delta: number) => {
    if (closing) return;
    const np = Math.max(0, Math.min(cur + delta, pages - 1));
    if (np === cur) return;
    setDir(delta > 0 ? "left" : "right");
    setPage(np);
  };

  const onWheel = (e: WheelEvent) => {
    if (closing) return;
    const now = Date.now();
    if (now - lastWheel.current < 250) return;
    const d = Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY;
    if (Math.abs(d) < 15) return;
    lastWheel.current = now;
    go(d > 0 ? 1 : -1);
  };

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;
    const oldIndex = items.findIndex((e) => e.path === active.id);
    const newIndex = items.findIndex((e) => e.path === over.id);
    if (oldIndex < 0 || newIndex < 0) return;
    const reordered = arrayMove(items, oldIndex, newIndex);
    setItems(reordered);
    void Promise.all(
      reordered.map((e, i) => setItemMeta(folder.path, e.file_name, undefined, i)),
    );
  };

  return (
    <div
      className={`folder-popup-overlay${closing ? " closing" : ""}`}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) close();
      }}
    >
      <div
        className={`folder-popup${closing ? " closing" : ""}`}
        onMouseDown={(e) => {
          if (e.target === e.currentTarget) close();
        }}
        onWheel={onWheel}
      >
        <div className="folder-popup-header">
          <span>{folder.name}</span>
          <button className="folder-close" title="关闭" onClick={close}>
            ✕
          </button>
        </div>

        <div className="folder-popup-body">
          {pages > 1 && (
            <button
              className="page-arrow"
              disabled={cur === 0}
              onClick={() => go(-1)}
              title="上一页"
            >
              ‹
            </button>
          )}
          <div
            key={cur}
            className={`folder-popup-grid slide-${dir}`}
            onMouseDown={(e) => {
              if (e.target === e.currentTarget) close();
            }}
          >
            <DndContext
              sensors={sensors}
              collisionDetection={closestCenter}
              onDragEnd={handleDragEnd}
            >
              <SortableContext
                items={pageItems.map((e) => e.path)}
                strategy={rectSortingStrategy}
              >
                {cells.map((e, i) =>
                  e ? (
                    <SortableItem key={e.path} entry={e} onOpen={onOpen} />
                  ) : (
                    <div key={i} className="card-slot" />
                  ),
                )}
              </SortableContext>
            </DndContext>
          </div>
          {pages > 1 && (
            <button
              className="page-arrow"
              disabled={cur === pages - 1}
              onClick={() => go(1)}
              title="下一页"
            >
              ›
            </button>
          )}
        </div>

        {pages > 1 && (
          <div className="page-dots">
            {Array.from({ length: pages }).map((_, i) => (
              <span
                key={i}
                className={`dot${i === cur ? " active" : ""}`}
                onClick={() => {
                  setDir(i > cur ? "left" : "right");
                  setPage(i);
                }}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
