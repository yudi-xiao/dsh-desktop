// Smoke test for the dsh runtime: launches `dsh web`, waits for the readiness
// line, checks the served origin responds, then terminates the process tree
// cleanly. Run after every @deepseek-ai/dsh upgrade (and in CI) to catch
// breaking changes before they reach users.
//
//   node scripts/smoke-runtime.mjs
//
// Exit 0 when the runtime boots and serves; nonzero otherwise.
import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const launcher = process.env.DSH_DESKTOP_SMOKE_LAUNCHER ||
  join(root, "apps", "runtime", "dsh-web.mjs");

if (!existsSync(launcher)) {
  console.error(`missing launcher: ${launcher}`);
  process.exit(1);
}

const node = process.env.DSH_DESKTOP_SMOKE_NODE || process.execPath;
const dshHome = process.env.DSH_DESKTOP_SMOKE_HOME ||
  mkdtempSync(join(tmpdir(), "dsh-desktop-smoke-"));
writeFileSync(
  join(dshHome, "codex-usage.json"),
  JSON.stringify({
    backend: "codex",
    phase: "ready",
    cliVersion: "0.147.0",
    requiredCliVersion: "0.147.0",
    appServerRunning: true,
    authMode: "chatgpt",
    managedInstall: true,
    state: "ready",
    planType: "smoke",
    email: "smoke@example.invalid",
    updatedAt: 1,
    buckets: [{ id: "smoke", primary: { usedPercent: 25, remainingPercent: 75 } }],
    credits: null,
    resetCredits: null,
    accountUsage: null,
    threadUsage: {},
    error: null,
  }),
);
const child = spawn(node, [launcher], {
  stdio: ["ignore", "pipe", "inherit"],
  env: {
    ...process.env,
    DSH_HOME: dshHome,
    DSH_DESKTOP_CODEX_USAGE_FILE: join(dshHome, "codex-usage.json"),
  },
});

let ready = null;
let buffer = "";
const timeout = setTimeout(() => fail("timed out waiting for readiness"), 60_000);

child.stdout.on("data", (chunk) => {
  buffer += chunk.toString();
  const match = buffer.match(/dsh web: (http:\/\/127\.0\.0\.1:\d+)/);
  if (match && !ready) {
    ready = match[1];
    onReady(ready);
  }
});

async function onReady(url) {
  console.log(`ready: ${url}`);
  try {
    const res = await fetch(url);
    if (!res.ok) {
      fail(`origin returned HTTP ${res.status}`);
      return;
    }
    const html = await res.text();
    if (!html.includes("@dsh-desktop/dsh-codex-usage")) {
      fail("Codex usage browser module is missing from the dsh boot manifest");
      return;
    }
    console.log(`origin: HTTP ${res.status} OK`);
    const bundleRes = await fetch(
      `${url}/plugins/@dsh-desktop/dsh-codex-usage/client.js`,
    );
    const bundle = bundleRes.ok ? await bundleRes.text() : "";
    if (
      !bundleRes.ok ||
      !bundle.includes("UsageSettings") ||
      !bundle.includes("CodexControl") ||
      !bundle.includes("CodexSessionDrawer") ||
      !bundle.includes("CodexComposer") ||
      !bundle.includes("conversation.composer") ||
      !bundle.includes("conversation.session.header.actions") ||
      !bundle.includes("codex_model_catalog") ||
      !bundle.includes("codex_collaboration_mode_catalog") ||
      !bundle.includes("codex_session_send") ||
      !bundle.includes("codex_session_goal_update") ||
      !bundle.includes("codex_session_goal_clear") ||
      !bundle.includes("codex_session_index") ||
      !bundle.includes("startCodexSession") ||
      !bundle.includes("workspace session projection") ||
      !bundle.includes("summary.blank = false") ||
      !bundle.includes("AttachmentPicker") ||
      !bundle.includes("CodexConversationView") ||
      !bundle.includes("CodexComposerCard") ||
      !bundle.includes("CodexModeControl") ||
      !bundle.includes("CodexGoalBar") ||
      !bundle.includes("dcu-codexShell") ||
      !bundle.includes("dcu-modeRow") ||
      !bundle.includes("item/plan/delta") ||
      !bundle.includes("chat.current.scrollTop = chat.current.scrollHeight") ||
      !bundle.includes("usageStatus !== lastUsageStatus.current") ||
      !bundle.includes("activeTurnId && event.params?.turnId === activeTurnId") ||
      !bundle.includes("dataBase64") ||
      !bundle.includes(".dcu-add{") ||
      !bundle.includes("codex_status_cached") ||
      !bundle.includes("codex_logout_chatgpt") ||
      !bundle.includes(".dcu-grid>*{min-width:0}") ||
      !bundle.includes(".dcu-history i{flex:1 1 0;min-width:0;")
    ) {
      fail("Codex usage browser module could not be served");
      return;
    }
    console.log(`Codex usage/session adapter UI: HTTP ${bundleRes.status} OK`);
    const usageRes = await fetch(`${url}/desktop-api/codex/usage`);
    if (!usageRes.ok) {
      fail(`Codex usage endpoint returned HTTP ${usageRes.status}`);
      return;
    }
    const usage = await usageRes.json();
    if (
      usage.backend !== "codex" ||
      usage.state !== "ready" ||
      usage.email !== "smoke@example.invalid" ||
      usage.buckets?.[0]?.primary?.remainingPercent !== 75
    ) {
      fail("Codex usage endpoint did not return the on-disk snapshot");
      return;
    }
    console.log(`Codex usage bridge: HTTP ${usageRes.status} OK`);
    // Readiness is emitted before every Cordis plugin has necessarily settled.
    // Keep the child alive long enough to catch late module-resolution crashes
    // that would otherwise leave the desktop shell stuck on its placeholder.
    await new Promise((resolve) => setTimeout(resolve, 10_000));
    if (child.exitCode !== null) {
      fail(`dsh web exited after readiness (code ${child.exitCode})`);
      return;
    }
    const stableRes = await fetch(url);
    if (!stableRes.ok) {
      fail(`origin became unhealthy after readiness: HTTP ${stableRes.status}`);
      return;
    }
    console.log("stability: process alive and origin healthy after 10s");
    cleanup(0);
  } catch (err) {
    fail(`origin fetch failed: ${err.message}`);
  }
}

function cleanup(code) {
  clearTimeout(timeout);
  // Kill the whole tree (node → dsh bin.js) so no orphan remains.
  if (process.platform === "win32") {
    spawn("taskkill", ["/T", "/F", "/PID", String(child.pid)], {
      stdio: "ignore",
    });
  } else {
    child.kill("SIGTERM");
  }
  setTimeout(() => process.exit(code), 200);
}

function fail(msg) {
  clearTimeout(timeout);
  console.error(`SMOKE FAIL: ${msg}`);
  cleanup(1);
}

child.on("exit", (code) => {
  if (!ready) {
    clearTimeout(timeout);
    console.error(`SMOKE FAIL: dsh web exited before ready (code ${code})`);
    process.exit(1);
  }
});
