#!/usr/bin/env node
// Smoke test for the pinned Codex app-server protocol. It performs only the
// initialize/initialized/account-read sequence, verifies the dynamic model
// catalog used by the composer, and, for a managed ChatGPT account, verifies
// the read-only rate-limit surface used by the desktop UI.
// It never starts a login or a turn. Pass --dlx to download and test the pinned
// release through pnpm.

import { spawn } from "node:child_process";
import readline from "node:readline";

const CODEX_VERSION = "0.147.0";
const useDlx = process.argv.includes("--dlx");
const useWindowsCommandShim = process.platform === "win32";
const program = useWindowsCommandShim
  ? process.env.ComSpec || "cmd.exe"
  : useDlx
    ? "pnpm"
    : "codex";
const codexArgs = useDlx
  ? ["dlx", `@openai/codex@${CODEX_VERSION}`, "app-server", "--listen", "stdio://"]
  : ["app-server", "--listen", "stdio://"];
// CreateProcess cannot execute pnpm.cmd directly. Keep the command string
// entirely constant so cmd.exe never receives user-controlled input.
const args = useWindowsCommandShim
  ? ["/d", "/s", "/c", `${useDlx ? "pnpm " : "codex "}${codexArgs.join(" ")}`]
  : codexArgs;

const child = spawn(program, args, {
  stdio: ["pipe", "pipe", "inherit"],
  shell: false,
});
const lines = readline.createInterface({ input: child.stdout });
let initialized = false;
let finished = false;
let authType = "none";
let selectableModels = [];

function send(message) {
  child.stdin.write(`${JSON.stringify(message)}\n`);
}

function finish(code, message) {
  if (finished) return;
  finished = true;
  clearTimeout(timeout);
  if (message) console.log(message);
  lines.close();
  child.kill();
  process.exitCode = code;
}

const timeout = setTimeout(() => finish(1, "Codex app-server smoke test timed out"), 30_000);

child.on("error", (error) => finish(1, `Failed to start Codex app-server: ${error.message}`));
child.on("exit", (code) => {
  if (!finished) finish(code || 1, `Codex app-server exited before handshake (${code})`);
});

lines.on("line", (line) => {
  let message;
  try {
    message = JSON.parse(line);
  } catch {
    return;
  }
  if (message.id === 1 && message.result && !initialized) {
    initialized = true;
    send({ method: "initialized", params: {} });
    send({ method: "account/read", id: 2, params: { refreshToken: false } });
    return;
  }
  if (message.id === 1 && message.error) {
    finish(1, `initialize failed: ${message.error.message}`);
    return;
  }
  if (message.id === 2) {
    if (message.error) {
      finish(1, `account/read failed: ${message.error.message}`);
      return;
    }
    authType = message.result?.account?.type ?? "none";
    send({ method: "model/list", id: 3, params: { limit: 100, includeHidden: false } });
    return;
  }
  if (message.id === 3) {
    if (message.error) {
      finish(1, `model/list failed: ${message.error.message}`);
      return;
    }
    if (!Array.isArray(message.result?.data) || !message.result.data.some((model) => model?.model)) {
      finish(1, "model/list returned no selectable models");
      return;
    }
    selectableModels = message.result.data;
    send({ method: "collaborationMode/list", id: 4, params: {} });
    return;
  }
  if (message.id === 4) {
    if (message.error) {
      finish(1, `collaborationMode/list failed: ${message.error.message}`);
      return;
    }
    const modes = message.result?.data;
    if (!Array.isArray(modes) || !["default", "plan"].every((mode) => modes.some((entry) => entry?.mode === mode))) {
      finish(1, "collaborationMode/list did not return default and plan modes");
      return;
    }
    if (authType === "chatgpt") {
      send({ method: "account/rateLimits/read", id: 5, params: {} });
    } else {
      finish(0, `Codex app-server ${CODEX_VERSION} ready (auth: ${authType}; ${selectableModels.length} models; default/plan modes ready)`);
    }
    return;
  }
  if (message.id === 5) {
    if (message.error) {
      finish(1, `account/rateLimits/read failed: ${message.error.message}`);
      return;
    }
    const buckets = message.result?.rateLimitsByLimitId;
    if (!message.result?.rateLimits && (!buckets || typeof buckets !== "object")) {
      finish(1, "account/rateLimits/read returned no rate-limit buckets");
      return;
    }
    // Current documentation includes this method, while the generated 0.147.0
    // schema may not. The product treats method-not-found as optional history.
    send({ method: "account/usage/read", id: 6, params: {} });
    return;
  }
  if (message.id === 6) {
    const history = message.error ? "optional history unavailable" : "history available";
    finish(
      0,
      `Codex app-server ${CODEX_VERSION} ready (auth: ${authType}; ${selectableModels.length} models; default/plan modes ready; rate limits ready; ${history})`,
    );
  }
});

send({
  method: "initialize",
  id: 1,
  params: {
    clientInfo: {
      name: "dsh_desktop_smoke",
      title: "DSH Desktop Smoke Test",
      version: "0.1.0",
    },
    capabilities: { experimentalApi: true },
  },
});
