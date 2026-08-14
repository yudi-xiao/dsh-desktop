import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

type Status = "connecting" | "error" | "stopped";

// Connecting placeholder shown before the dsh web child process reports ready.
// Once ready, the Rust supervisor navigates this window to the served loopback
// origin. Here we only surface backend failures and restarts.
export default function App() {
  const [status, setStatus] = useState<Status>("connecting");
  const [message, setMessage] = useState("");

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];

    listen<string>("harness-error", (event) => {
      if (!disposed) {
        setStatus("error");
        setMessage(event.payload);
      }
    }).then((unlisten) => unlisteners.push(unlisten));

    listen("harness-stopped", () => {
      if (!disposed) {
        setStatus("stopped");
        setMessage("");
      }
    }).then((unlisten) => unlisteners.push(unlisten));

    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, []);

  return (
    <main className="shell">
      <div className="card">
        <div className="spinner" aria-hidden="true" />
        <h1>DSH Desktop</h1>
        {status === "connecting" && <p>正在启动 DeepSeek Harness…</p>}
        {status === "error" && (
          <p className="error">
            后端启动失败，正在重试。
            {message ? <span className="detail">{message}</span> : null}
          </p>
        )}
        {status === "stopped" && <p>后端已停止，正在重新连接…</p>}
      </div>
    </main>
  );
}
