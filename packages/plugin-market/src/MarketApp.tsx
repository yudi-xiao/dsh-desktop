import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { PluginEntry, PluginPreview } from "./types";
import "./MarketApp.css";

// Built-in catalog. A remote catalog (e.g. a dsh-suite-compatible plugins.json)
// can be layered in later; the transaction commands are catalog-agnostic.
const CATALOG: PluginEntry[] = [
  {
    name: "dsh-spotlight",
    repo: "0xsline/dsh-spotlight",
    description: "键盘优先的命令面板，快速跳转会话与操作。",
    category: "UI",
  },
  {
    name: "dsh-at-file",
    repo: "omdsh-dev/dsh-at-file",
    description: "Codex 风格 @file 提及：搜索工作区文件并附加到提示词。",
    category: "UI",
  },
  {
    name: "dsh-visualize",
    repo: "Nagi-ovo/dsh-visualize",
    description: "会话内生成式 UI：模型渲染交互式 HTML 卡片。",
    category: "UI",
  },
  {
    name: "dsh-share",
    repo: "hellodigua/dsh-share",
    description: "一键分享你的会话。",
    category: "Sessions",
  },
  {
    name: "dsh-tool-approval",
    repo: "ilharp/dsh-tool-approval",
    description: "手动批准模式（Manual Mode / Ask Mode）。",
    category: "Tools",
  },
];

export function MarketApp() {
  const [query, setQuery] = useState("");
  const [busy, setBusy] = useState(false);
  const [preview, setPreview] = useState<PluginPreview | null>(null);
  const [hasCandidate, setHasCandidate] = useState(false);
  const [hasPrevious, setHasPrevious] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState(false);

  const refreshState = useCallback(async () => {
    try {
      setHasCandidate(await invoke<boolean>("marketplace_has_candidate"));
      setHasPrevious(await invoke<boolean>("marketplace_has_previous"));
    } catch {
      // Not running under the desktop shell (plain browser preview).
    }
  }, []);

  useEffect(() => {
    refreshState();
  }, [refreshState]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return CATALOG;
    return CATALOG.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        p.description.toLowerCase().includes(q) ||
        p.category.toLowerCase().includes(q),
    );
  }, [query]);

  const install = async (entry: PluginEntry) => {
    setBusy(true);
    setError(false);
    setMessage(`正在安装 ${entry.name} 到候选 Profile…`);
    try {
      await invoke("marketplace_install", { spec: entry.name });
      setMessage(`已安装 ${entry.name} 到候选 Profile，请预览并应用。`);
      setHasCandidate(true);
      setPreview(await invoke<PluginPreview>("marketplace_preview"));
    } catch (e) {
      setError(true);
      setMessage(`安装失败：${e}`);
    } finally {
      setBusy(false);
    }
  };

  const apply = async () => {
    setBusy(true);
    setError(false);
    try {
      await invoke("marketplace_apply");
      setMessage("已应用。重启 dsh 后插件生效。");
      setHasCandidate(false);
      setHasPrevious(true);
      setPreview(null);
    } catch (e) {
      setError(true);
      setMessage(`应用失败：${e}`);
    } finally {
      setBusy(false);
    }
  };

  const undo = async () => {
    setBusy(true);
    setError(false);
    try {
      await invoke("marketplace_undo");
      setMessage("已回滚到上一份 Profile。");
      setHasPrevious(false);
      setPreview(null);
    } catch (e) {
      setError(true);
      setMessage(`回滚失败：${e}`);
    } finally {
      setBusy(false);
    }
  };

  const reset = async () => {
    setBusy(true);
    setError(false);
    try {
      await invoke("marketplace_reset");
      setMessage("已放弃候选 Profile，桌面保持不变。");
      setHasCandidate(false);
      setPreview(null);
    } catch (e) {
      setError(true);
      setMessage(`放弃失败：${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <main className="market">
      <header className="market-head">
        <h1>插件市场</h1>
        <p>安装、隔离预览、应用与回滚 DeepSeek Harness 插件。</p>
      </header>

      <input
        className="search"
        type="search"
        placeholder="搜索插件…"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />

      <ul className="catalog">
        {filtered.map((p) => (
          <li key={p.name} className="plugin">
            <div className="plugin-meta">
              <span className="plugin-name">{p.name}</span>
              <span className="plugin-cat">{p.category}</span>
              <span className="plugin-repo">{p.repo}</span>
              <p className="plugin-desc">{p.description}</p>
            </div>
            <button disabled={busy} onClick={() => install(p)}>
              安装
            </button>
          </li>
        ))}
        {filtered.length === 0 && <li className="empty">无匹配插件</li>}
      </ul>

      {preview && (
        <section className="preview">
          <h2>变更预览（候选 vs 活动）</h2>
          <div className="preview-grid">
            <div>
              <h3>新增依赖</h3>
              <ul>
                {preview.added.map((d) => (
                  <li key={d}>{d}</li>
                ))}
                {preview.added.length === 0 && <li className="muted">无</li>}
              </ul>
            </div>
            <div>
              <h3>移除依赖</h3>
              <ul>
                {preview.removed.map((d) => (
                  <li key={d}>{d}</li>
                ))}
                {preview.removed.length === 0 && <li className="muted">无</li>}
              </ul>
            </div>
          </div>
          <pre className="patch-diff">{preview.patch_diff}</pre>
        </section>
      )}

      <footer className="actions">
        {hasCandidate && (
          <>
            <button className="primary" disabled={busy} onClick={apply}>
              应用
            </button>
            <button disabled={busy} onClick={reset}>
              放弃
            </button>
          </>
        )}
        {hasPrevious && (
          <button disabled={busy} onClick={undo}>
            回滚
          </button>
        )}
      </footer>

      {message && <p className={error ? "msg error" : "msg"}>{message}</p>}
    </main>
  );
}
