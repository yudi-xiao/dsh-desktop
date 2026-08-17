import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./BoardApp.css";

// Mirrors the Rust `WorkspaceCard` (camelCase serde projection).
interface WorkspaceCard {
  id: string;
  title: string;
  path: string;
  sessionCount: number;
  archivedSessionCount: number;
  status: "active" | "empty" | "archived";
  createdAt: string;
  updatedAt: string;
}

const STATUS_LABEL: Record<WorkspaceCard["status"], string> = {
  active: "活跃",
  empty: "空",
  archived: "已归档",
};

function formatDate(iso: string): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function BoardApp() {
  const [cards, setCards] = useState<WorkspaceCard[] | null>(null);
  const [error, setError] = useState("");

  const refresh = useCallback(async () => {
    try {
      setError("");
      setCards(await invoke<WorkspaceCard[]>("board_list_workspaces"));
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const openPath = async (path: string) => {
    try {
      await invoke("board_open_path", { path });
    } catch (e) {
      setError(`打开目录失败：${e}`);
    }
  };

  return (
    <main className="board">
      <header className="board-head">
        <div>
          <h1>项目看板</h1>
          <p>来自 DeepSeek Harness 工作区注册表的项目概览。</p>
        </div>
        <button className="refresh" onClick={refresh}>
          刷新
        </button>
      </header>

      {error && <p className="board-error">{error}</p>}

      {cards === null && !error && <p className="board-hint">正在加载…</p>}

      {cards !== null && cards.length === 0 && (
        <div className="board-empty">
          <p>还没有项目。</p>
          <p className="muted">在 dsh 的 Web UI 中「选择工作区」添加项目目录后，这里会列出它们。</p>
        </div>
      )}

      {cards !== null && cards.length > 0 && (
        <ul className="board-grid">
          {cards.map((c) => (
            <li key={c.id} className="card">
              <div className="card-top">
                <span className={`status status-${c.status}`}>{STATUS_LABEL[c.status]}</span>
                <span className="card-title" title={c.title}>
                  {c.title}
                </span>
              </div>
              <p className="card-path" title={c.path}>
                {c.path}
              </p>
              <div className="card-meta">
                <span>
                  会话 {c.sessionCount}
                  {c.archivedSessionCount > 0 ? `（归档 ${c.archivedSessionCount}）` : ""}
                </span>
                <span>更新于 {formatDate(c.updatedAt)}</span>
              </div>
              <div className="card-actions">
                <button onClick={() => openPath(c.path)}>打开目录</button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </main>
  );
}
