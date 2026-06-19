import { memo } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Entry } from "../types";
import FolderPreview from "./FolderPreview";

interface Props {
  entry: Entry;
  onOpen: (e: Entry) => void;
  onContextMenu: (e: Entry, x: number, y: number) => void;
  disabled?: boolean;
}

function EntryCardBase({ entry, onOpen, onContextMenu, disabled }: Props) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: entry.path, disabled });

  const icon = entry.icon ? convertFileSrc(entry.icon) : null;

  // 拆成两层：外层 card-slot 承载 dnd-kit 的 transform（拖拽位移），
  // 内层 .card 做 hover 放大——避免 dnd-kit 的 inline transform 覆盖 hover scale。
  return (
    <div
      ref={setNodeRef}
      className={`card-slot${isDragging ? " dragging" : ""}`}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      onContextMenu={(e) => {
        e.preventDefault();
        onContextMenu(entry, e.clientX, e.clientY);
      }}
      {...attributes}
      {...listeners}
    >
      <button
        className={`card${entry.kind === "folder" ? " card-folder" : ""}`}
        onClick={() => onOpen(entry)}
        title={entry.name}
      >
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
      </button>
    </div>
  );
}

const EntryCard = memo(EntryCardBase);
export default EntryCard;
