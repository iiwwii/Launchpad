import { useEffect, useRef } from "react";

interface Props {
  x: number;
  y: number;
  canDelete: boolean;
  onOpen: () => void;
  onRename: () => void;
  onReveal: () => void;
  onDelete: () => void;
  onClose: () => void;
}

export default function ContextMenu({
  x,
  y,
  canDelete,
  onOpen,
  onRename,
  onReveal,
  onDelete,
  onClose,
}: Props) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const onDown = (e: globalThis.MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  return (
    <div
      ref={ref}
      className="ctx-menu"
      style={{ left: x, top: y }}
      onMouseDown={(e) => e.stopPropagation()}
    >
      <button onClick={() => { onOpen(); onClose(); }}>打开</button>
      <button onClick={() => { onRename(); onClose(); }}>重命名</button>
      <button onClick={() => { onReveal(); onClose(); }}>在文件夹中显示</button>
      {canDelete && (
        <button
          className="danger"
          onClick={() => { onDelete(); onClose(); }}
        >
          删除快捷方式
        </button>
      )}
    </div>
  );
}
