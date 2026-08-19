#!/usr/bin/env node
// Launcher for `dsh web`: runs the pinned @deepseek-ai/dsh CLI as a child and
// forwards signals + exit codes so the desktop supervisor manages one process
// tree (no orphaned node). The packaged app runs this under the bundled
// portable Node binary; dev runs it under the system Node.
//
// Defaults mirror the desktop shell's posture: loopback-only, OS-assigned port.
import { spawn } from "node:child_process";
import { createRequire } from "node:module";
import {
  copyFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  realpathSync,
  rmdirSync,
  symlinkSync,
  unlinkSync,
} from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const dshBin = join(here, "node_modules", "@deepseek-ai", "dsh", "lib", "bin.js");
const desktopPlugin = join(
  here,
  "node_modules",
  "@dsh-desktop",
  "dsh-codex-usage",
);
const desktopPatch = join(desktopPlugin, "cordis.patch.yml");

// Copy only package-owned files. A persistent dsh profile may add a
// node_modules directory below this package; recursively replacing the whole
// directory races with pnpm/Node on Windows and can fail with EIO/access denied.
function syncDesktopPlugin(source, target) {
  mkdirSync(target, { recursive: true });
  for (const entry of readdirSync(source, { withFileTypes: true })) {
    if (entry.name === "node_modules") continue;
    const from = join(source, entry.name);
    const to = join(target, entry.name);
    if (entry.isDirectory()) syncDesktopPlugin(from, to);
    else if (entry.isFile()) {
      // pnpm may already have hard-linked the profile copy back to the
      // packaged closure. Copying a file onto the same Windows file identity
      // throws EPERM, so identical bytes are deliberately left in place.
      const unchanged = existsSync(to) && readFileSync(from).equals(readFileSync(to));
      if (!unchanged) copyFileSync(from, to);
    }
  }
}

function removePackageLink(target) {
  try {
    const metadata = lstatSync(target);
    if (!metadata.isSymbolicLink()) return false;
    if (process.platform === "win32") rmdirSync(target);
    else unlinkSync(target);
    return true;
  } catch (error) {
    if (error?.code === "ENOENT") return false;
    throw error;
  }
}

function ensurePackageLink(source, target) {
  const sourcePath = realpathSync(source);
  try {
    const metadata = lstatSync(target);
    // Preserve real directories (including packages installed by the user),
    // but reconcile application-owned links on every boot. An upgrade leaves
    // the old extracted runtime alive until pruning; merely checking
    // existsSync(target) therefore accepts a live link that will disappear
    // while dsh is still resolving its plugin tree.
    if (!metadata.isSymbolicLink()) return;
    try {
      const targetPath = realpathSync(target);
      const normalize = (value) => process.platform === "win32"
        ? value.toLowerCase()
        : value;
      if (normalize(targetPath) === normalize(sourcePath)) return;
    } catch {
      // A dangling Windows junction can surface as UNKNOWN (-4094), rather
      // than ENOENT, when realpath follows its missing target. This fallback
      // tree is application-owned, so any link that cannot be resolved is
      // unusable by Node and must be recreated from the current runtime.
    }
    removePackageLink(target);
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
  mkdirSync(dirname(target), { recursive: true });
  symlinkSync(
    sourcePath,
    target,
    process.platform === "win32" ? "junction" : "dir",
  );
}

function syncProfileFallback(sourceRoot, targetRoot) {
  mkdirSync(targetRoot, { recursive: true });
  for (const entry of readdirSync(sourceRoot, { withFileTypes: true })) {
    if (entry.name.startsWith(".")) continue;
    const source = join(sourceRoot, entry.name);
    if (entry.name.startsWith("@") && entry.isDirectory()) {
      const scopeTarget = join(targetRoot, entry.name);
      mkdirSync(scopeTarget, { recursive: true });
      for (const packageEntry of readdirSync(source, { withFileTypes: true })) {
        if (!packageEntry.name.startsWith(".")) {
          ensurePackageLink(
            join(source, packageEntry.name),
            join(scopeTarget, packageEntry.name),
          );
        }
      }
    } else if (entry.isDirectory() || entry.isSymbolicLink()) {
      ensurePackageLink(source, join(targetRoot, entry.name));
    }
  }
}

function packageDirFromAnchor(anchor, packageName) {
  for (const searchRoot of createRequire(anchor).resolve.paths(packageName) ?? []) {
    const candidate = join(searchRoot, packageName);
    if (existsSync(join(candidate, "package.json"))) return candidate;
  }
  return undefined;
}

// Reproduce dsh's dependency-closure traversal, but write it into the
// profile-local node_modules directory that dsh itself deliberately leaves
// alone. The workspace uses pnpm's isolated layout, so scanning only the
// launcher's top-level node_modules would miss most UI packages.
function syncPackageClosure(installAnchor, targetRoot) {
  // Match Node's default symlink-following module identity. In a pnpm
  // workspace the visible package path is a junction, while its dependencies
  // are resolvable only beside the real store path.
  installAnchor = realpathSync(installAnchor);
  const appManifest = JSON.parse(readFileSync(installAnchor, "utf8"));
  const packages = new Map();
  if (appManifest.name) packages.set(appManifest.name, dirname(installAnchor));
  const queue = [{ anchor: installAnchor, manifest: appManifest }];

  for (let next = queue.shift(); next; next = queue.shift()) {
    const dependencies = [
      ...Object.keys(next.manifest.dependencies ?? {}),
      ...Object.keys(next.manifest.peerDependencies ?? {}),
    ];
    for (const dependency of dependencies) {
      if (packages.has(dependency)) continue;
      const packageDir = packageDirFromAnchor(next.anchor, dependency);
      if (!packageDir) continue;
      packages.set(dependency, packageDir);
      const manifestPath = join(packageDir, "package.json");
      queue.push({
        anchor: manifestPath,
        manifest: JSON.parse(readFileSync(manifestPath, "utf8")),
      });
    }
  }

  for (const [packageName, packageDir] of packages) {
    ensurePackageLink(packageDir, join(targetRoot, packageName));
  }
}

// dsh resolves Loader entries relative to its generated profile under
// DSH_HOME. Mirror this tiny first-party package into that resolution tree;
// the packaged runtime remains the source of truth and overwrites its files on
// every start. No third-party dependencies or credentials are copied.
if (process.env.DSH_HOME && existsSync(desktopPlugin)) {
  const desktopPluginTarget = join(
    process.env.DSH_HOME,
    "node_modules",
    "@dsh-desktop",
    "dsh-codex-usage",
  );
  mkdirSync(dirname(desktopPluginTarget), { recursive: true });
  // pnpm represents workspace packages as junctions on Windows. Resolve the
  // source first and update package-owned files without touching the profile's
  // dependency directory.
  syncDesktopPlugin(realpathSync(desktopPlugin), desktopPluginTarget);
}

// The profile imports packages using Node's parent-directory lookup. dsh owns
// <DSH_HOME>/profiles/node_modules and may replace its junctions when an
// installation moves (which every desktop runtime extraction does). Keep that
// shared fallback complete, but also maintain a profile-local fallback that dsh
// does not heal. The local layer wins Node resolution and stays stable while
// the shared layer is being reconciled. Existing real directories and
// user-installed packages are preserved in both locations.
if (process.env.DSH_HOME) {
  const bundledModules = join(here, "node_modules");
  const profilesRoot = join(process.env.DSH_HOME, "profiles");
  syncProfileFallback(bundledModules, join(profilesRoot, "node_modules"));
  syncPackageClosure(
    join(here, "node_modules", "@deepseek-ai", "dsh", "package.json"),
    join(profilesRoot, "web", "node_modules"),
  );
  if (existsSync(desktopPlugin)) {
    syncPackageClosure(
      join(realpathSync(desktopPlugin), "package.json"),
      join(profilesRoot, "web", "node_modules"),
    );
  }
}

const args = [
  "web",
  "--patch",
  desktopPatch,
  "--host",
  "127.0.0.1",
  "--port",
  "0",
  ...process.argv.slice(2),
];

const child = spawn(process.execPath, [dshBin, ...args], {
  stdio: "inherit",
  env: process.env,
  // The parent Tauri process is a Windows GUI application. Keep this nested
  // Node CLI invisible as well as the launcher spawned directly from Rust.
  windowsHide: true,
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
