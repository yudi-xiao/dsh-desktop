// Prepares the self-contained runtime under vendor/runtime/<target>/:
//   node/  — portable Node binary (official nodejs.org distribution)
//   app/   — @dsh-desktop/runtime closure (pnpm deploy: dsh + production deps)
//
// Run on each target OS before packaging (native modules are platform-specific):
//   node scripts/prepare-runtime.mjs
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  rmSync,
  renameSync,
  writeFileSync,
} from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const NODE_VERSION = "22.19.0";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");

function nodeDist() {
  const { platform, arch } = process;
  const dirs = {
    "win32-x64": "win-x64",
    "darwin-x64": "darwin-x64",
    "darwin-arm64": "darwin-arm64",
    "linux-x64": "linux-x64",
    "linux-arm64": "linux-arm64",
  };
  const dir = dirs[`${platform}-${arch}`];
  if (!dir) throw new Error(`unsupported platform ${platform}-${arch}`);
  const ext = platform === "win32" ? "zip" : platform === "darwin" ? "tar.gz" : "tar.xz";
  return { dir, ext, platform };
}

async function downloadNode(targetDir) {
  const { dir, ext, platform } = nodeDist();
  const nodeBin = join(targetDir, "node", platform === "win32" ? "node.exe" : "node");
  if (existsSync(nodeBin)) {
    console.log("portable node already present, skipping download");
    return;
  }

  const archiveName = `node-v${NODE_VERSION}-${dir}.${ext}`;
  const url = `https://nodejs.org/dist/v${NODE_VERSION}/${archiveName}`;
  const archivePath = join(targetDir, archiveName);
  const extractDir = join(targetDir, `node-v${NODE_VERSION}-${dir}`);

  console.log(`downloading ${url}`);
  const res = await fetch(url);
  if (!res.ok) throw new Error(`download failed: HTTP ${res.status}`);
  const buf = Buffer.from(await res.arrayBuffer());
  writeFileSync(archivePath, buf);
  console.log(`wrote ${archivePath} (${buf.length} bytes)`);

  console.log(`extracting ${archiveName}`);
  rmSync(extractDir, { recursive: true, force: true });
  let extract;
  if (platform === "win32") {
    // Windows ships GNU tar 1.35, which does not read zip; use PowerShell.
    extract = spawnSync(
      "powershell",
      [
        "-NoProfile",
        "-Command",
        `Expand-Archive -LiteralPath '${archiveName}' -DestinationPath '.' -Force`,
      ],
      { cwd: targetDir, stdio: "inherit" },
    );
  } else {
    // Relative archive name + cwd=targetDir avoids drive-letter host parsing.
    extract = spawnSync("tar", ["-xf", archiveName], {
      cwd: targetDir,
      stdio: "inherit",
    });
  }
  if (extract.status !== 0) throw new Error(`extraction failed`);

  rmSync(join(targetDir, "node"), { recursive: true, force: true });
  renameSync(extractDir, join(targetDir, "node"));
  rmSync(archivePath, { force: true });
  console.log("portable node ready");
}

function deployClosure(targetDir) {
  const appDir = join(targetDir, "app");
  console.log("deploying @dsh-desktop/runtime closure (hoisted install)");
  rmSync(appDir, { recursive: true, force: true });
  mkdirSync(appDir, { recursive: true });

  // The closure must be a flat (hoisted) node_modules: the isolated `.pnpm`
  // layout produces paths long enough to exceed the Windows NSIS installer's
  // path limit. Hoisted installs keep real top-level directories instead of
  // symlinks into `.pnpm/<pkg>@<ver>/`, shortening the longest path.
  copyFileSync(join(root, "apps/runtime/package.json"), join(appDir, "package.json"));
  copyFileSync(join(root, "apps/runtime/dsh-web.mjs"), join(appDir, "dsh-web.mjs"));

  const install = spawnSync(
    "pnpm",
    ["install", "--ignore-workspace", "--node-linker=hoisted"],
    { cwd: appDir, stdio: "inherit", shell: process.platform === "win32" },
  );
  if (install.status !== 0) throw new Error(`pnpm install failed`);
  console.log("closure ready");
}

// Archives the closure into a single `app.tar.gz` and removes the directory.
// Even a hoisted node_modules still nests on version conflicts, and those paths
// can exceed the Windows NSIS installer's path limit; a single archive keeps
// every bundled path short. The desktop shell extracts it at runtime into the
// user data directory (Node handles long paths there).
function archiveClosure(targetDir) {
  const appDir = join(targetDir, "app");
  const archive = join(targetDir, "app.tar.gz");
  console.log("archiving closure to app.tar.gz");
  rmSync(archive, { force: true });
  const tar = spawnSync("tar", ["-czf", "app.tar.gz", "-C", "app", "."], {
    cwd: targetDir,
    stdio: "inherit",
  });
  if (tar.status !== 0) throw new Error(`tar archive failed`);
  rmSync(appDir, { recursive: true, force: true });
  console.log("closure archived");
}

async function main() {
  const { dir } = nodeDist();
  const targetDir = join(root, "vendor", "runtime", dir);
  mkdirSync(targetDir, { recursive: true });

  await downloadNode(targetDir);
  deployClosure(targetDir);
  archiveClosure(targetDir);

  console.log(`\nruntime prepared at vendor/runtime/${dir}/`);
}

main().catch((err) => {
  console.error(err.message);
  process.exit(1);
});
