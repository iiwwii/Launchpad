import { useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { listEntries } from "../lib/api";
import type { Entry } from "../types";

/** 文件夹卡片预览：3×3 内部图标；超过 9 个时第 9 格堆叠小图标。 */
export default function FolderPreview({ path }: { path: string }) {
  const [items, setItems] = useState<Entry[]>([]);

  useEffect(() => {
    let cancelled = false;
    const reload = () => {
      listEntries(path)
        .then((list) => {
          if (!cancelled) setItems(list);
        })
        .catch(() => {});
    };
    reload();
    // 文件夹内容变化（加/删/排序）时刷新预览
    const unlisten = listen("launchpad:changed", reload);
    return () => {
      cancelled = true;
      void unlisten.then((fn) => fn());
    };
  }, [path]);

  const overflow = items.length > 9;
  const stackItems = items.slice(8, 11); // 第 9 格堆叠用

  const cells: (Entry | "overflow" | null)[] = [];
  for (let i = 0; i < 9; i++) {
    if (overflow && i === 8) cells.push("overflow");
    else if (i < items.length) cells.push(items[i]);
    else cells.push(null);
  }

  return (
    <div className="folder-preview">
      {cells.map((c, i) => {
        if (c === "overflow") {
          return (
            <div className="fp-cell fp-overflow" key={i}>
              {stackItems.map(
                (it, j) =>
                  it?.icon ? (
                    <img
                      key={j}
                      src={convertFileSrc(it.icon)}
                      className={`stack stack-${j}`}
                      alt=""
                      draggable={false}
                    />
                  ) : null,
              )}
            </div>
          );
        }
        if (c) {
          return (
            <div className="fp-cell" key={i}>
              {c.icon ? (
                <img src={convertFileSrc(c.icon)} alt="" draggable={false} />
              ) : null}
            </div>
          );
        }
        return <div className="fp-cell fp-empty" key={i} />;
      })}
    </div>
  );
}
