import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./CodexSettingsApp.css";

interface CodexStatus {
  backend: "dsh" | "codex";
  phase: string;
  installed: boolean;
  exactVersion: boolean;
  version: string | null;
  requiredVersion: string;
  dshVersion: string;
  executable: string | null;
  managedInstall: boolean;
  appServerRunning: boolean;
  authMode: string | null;
  planType: string | null;
  accountEmail: string | null;
  authUrl: string | null;
  error: string | null;
}

const PHASE_LABELS: Record<string, string> = {
  idle: "未启用",
  detecting: "正在检测 Codex CLI…",
  installing: "正在下载并安装 Codex CLI…",
  startingAppServer: "正在启动 Codex app-server…",
  checkingAccount: "正在检查 ChatGPT 登录状态…",
  startingLogin: "正在创建 ChatGPT 登录…",
  waitingForLogin: "等待浏览器完成 ChatGPT 登录…",
  signingOut: "正在退出 ChatGPT…",
  signedOut: "已退出 ChatGPT",
  ready: "Codex 已就绪",
  error: "配置失败",
};

function initialStatus(): CodexStatus {
  return {
    backend: "dsh",
    phase: "detecting",
    installed: false,
    exactVersion: false,
    version: null,
    requiredVersion: "0.147.0",
    dshVersion: "0.1.0-rc.6",
    executable: null,
    managedInstall: false,
    appServerRunning: false,
    authMode: null,
    planType: null,
    accountEmail: null,
    authUrl: null,
    error: null,
  };
}

export function CodexSettingsApp() {
  const [status, setStatus] = useState<CodexStatus>(initialStatus);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    listen<CodexStatus>("codex-status", (event) => {
      if (!disposed) setStatus(event.payload);
    }).then((dispose) => {
      unlisten = dispose;
    });
    invoke<CodexStatus>("codex_status")
      .then((value) => {
        if (!disposed) setStatus(value);
      })
      .finally(() => {
        if (!disposed) setLoading(false);
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const busy = useMemo(
    () =>
      [
        "detecting",
        "installing",
        "startingAppServer",
        "checkingAccount",
        "startingLogin",
        "signingOut",
      ].includes(status.phase),
    [status.phase],
  );

  const configureCodex = async () => {
    setStatus((value) => ({ ...value, backend: "codex", phase: "detecting", error: null }));
    await invoke("backend_select", { backend: "codex" });
  };

  const selectDsh = async () => {
    await invoke("backend_select", { backend: "dsh" });
  };

  const login = async () => {
    await invoke("codex_login_chatgpt");
  };

  return (
    <main className="backend-settings">
      <header>
        <p className="eyebrow">DSH Desktop</p>
        <h1>后端故障恢复</h1>
        <p className="lead">常规配置请使用 dsh Web UI 的「设置 → Codex」；此窗口仅用于主界面不可用时恢复后端。</p>
      </header>

      <section className={`backend-card ${status.backend === "dsh" ? "selected" : ""}`}>
        <div>
          <div className="card-title">
            <h2>DeepSeek Harness</h2>
            <span className="version">{status.dshVersion}</span>
          </div>
          <p>使用内置 dsh agent、会话存储与工具链。</p>
        </div>
        <button className="secondary" disabled={busy || status.backend === "dsh"} onClick={selectDsh}>
          {status.backend === "dsh" ? "当前使用" : "切换到 dsh"}
        </button>
      </section>

      <section className={`backend-card codex ${status.backend === "codex" ? "selected" : ""}`}>
        <div className="card-title">
          <h2>OpenAI Codex</h2>
          <span className="version">{status.requiredVersion}</span>
        </div>
        <p>自动检测 CLI；版本缺失或不匹配时安装到应用数据目录，随后启动 stdio app-server。</p>

        <dl className="status-grid">
          <div>
            <dt>CLI</dt>
            <dd className={status.exactVersion ? "good" : "muted"}>
              {status.installed ? `${status.version}${status.exactVersion ? " · 已匹配" : " · 将升级"}` : "未检测到"}
            </dd>
          </div>
          <div>
            <dt>App Server</dt>
            <dd className={status.appServerRunning ? "good" : "muted"}>
              {status.appServerRunning ? "运行中" : "未运行"}
            </dd>
          </div>
          <div>
            <dt>认证</dt>
            <dd className={status.authMode === "chatgpt" ? "good" : "muted"}>
              {status.authMode === "chatgpt"
                ? `ChatGPT${status.planType ? ` · ${status.planType}` : ""}`
                : status.authMode ?? "未登录"}
            </dd>
          </div>
          <div>
            <dt>来源</dt>
            <dd>{status.managedInstall ? "应用托管" : status.installed ? "系统 CLI" : "—"}</dd>
          </div>
          <div>
            <dt>账户</dt>
            <dd title={status.accountEmail ?? undefined}>{status.accountEmail ?? "—"}</dd>
          </div>
        </dl>

        <div className={`phase ${status.phase === "error" ? "phase-error" : ""}`}>
          {busy && <span className="spinner" aria-hidden="true" />}
          <span>{loading ? "正在读取状态…" : PHASE_LABELS[status.phase] ?? status.phase}</span>
        </div>

        {status.error && <pre className="error">{status.error}</pre>}
        {status.executable && <p className="path" title={status.executable}>{status.executable}</p>}

        <div className="actions">
          <button disabled={busy} onClick={configureCodex}>
            {status.backend === "codex" && status.phase === "ready" ? "重新检测" : "配置并启用 Codex"}
          </button>
          {status.appServerRunning && status.authMode !== "chatgpt" && (
            <button className="secondary" disabled={busy} onClick={login}>
              使用 ChatGPT 登录
            </button>
          )}
        </div>
      </section>

      <p className="footnote">
        Codex CLI 与 dsh 当前没有 LTS 通道；这里锁定 2026-08-17 核验的最新非 alpha 版本。启用后可在 dsh
        侧栏和「设置 → Codex」查看订阅限额与本地线程 Token 用量。
      </p>
    </main>
  );
}
