// Smoke test for the dsh runtime: launches `dsh web`, waits for the readiness
// line, checks the served origin responds, then terminates the process tree
// cleanly. Run after every @deepseek-ai/dsh upgrade (and in CI) to catch
// breaking changes before they reach users.
//
//   node scripts/smoke-runtime.mjs
//
// Exit 0 when the runtime boots and serves; nonzero otherwise.
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const launcher = join(root, "apps", "runtime", "dsh-web.mjs");

if (!existsSync(launcher)) {
  console.error(`missing launcher: ${launcher}`);
  process.exit(1);
}

const node = process.execPath;
const child = spawn(node, [launcher], { stdio: ["ignore", "pipe", "inherit"] });

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
    console.log(`origin: HTTP ${res.status} OK`);
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
