#!/usr/bin/env node
// Launcher for `dsh web`: runs the pinned @deepseek-ai/dsh CLI as a child and
// forwards signals + exit codes so the desktop supervisor manages one process
// tree (no orphaned node). The packaged app runs this under the bundled
// portable Node binary; dev runs it under the system Node.
//
// Defaults mirror the desktop shell's posture: loopback-only, OS-assigned port.
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const dshBin = join(here, "node_modules", "@deepseek-ai", "dsh", "lib", "bin.js");

const args = ["web", "--host", "127.0.0.1", "--port", "0", ...process.argv.slice(2)];

const child = spawn(process.execPath, [dshBin, ...args], {
  stdio: "inherit",
  env: process.env,
});

let forwarded = false;
for (const sig of ["SIGINT", "SIGTERM", "SIGHUP"]) {
  process.on(sig, () => {
    forwarded = true;
    child.kill(sig);
  });
}

child.on("error", (err) => {
  console.error(`[dsh-web] failed to start dsh: ${err.message}`);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (forwarded && signal) {
    // Mirror the signal as a conventional exit code.
    process.exitCode = 128 + (signal === "SIGINT" ? 2 : signal === "SIGTERM" ? 15 : 1);
    return;
  }
  process.exit(code ?? 1);
});
