// Prepares the self-contained runtime under vendor/runtime/<target>/:
//   node/  — portable Node binary (official nodejs.org distribution)
//   app/   — @dsh-desktop/runtime closure (pnpm deploy: dsh + production deps)
//
// Run on each target OS before packaging (native modules are platform-specific):
//   node scripts/prepare-runtime.mjs
import {
  cpSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
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
  const runtimePackage = JSON.parse(
    readFileSync(join(root, "apps/runtime/package.json"), "utf8"),
  );
  const pinnedDshVersion = runtimePackage.dependencies["@deepseek-ai/dsh"];
  const lockText = readFileSync(join(root, "pnpm-lock.yaml"), "utf8");
  const escapedVersion = pinnedDshVersion.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const packagePattern = new RegExp(
    `^  '(@deepseek-ai/dsh(?:-[^@']+)?)@${escapedVersion}(?:\\([^']*\\))?':$`,
    "gm",
  );
  const pinnedDshPackages = [...lockText.matchAll(packagePattern)].map((match) => match[1]);
  if (pinnedDshPackages.length < 50) {
    throw new Error(`could not derive the pinned dsh dependency set for ${pinnedDshVersion}`);
  }
  runtimePackage.pnpm = {
    ...(runtimePackage.pnpm || {}),
    overrides: Object.fromEntries(
      [...new Set(pinnedDshPackages)].map((name) => [name, pinnedDshVersion]),
    ),
  };
  runtimePackage.dependencies["@dsh-desktop/dsh-codex-usage"] =
    "file:./plugins/dsh-codex-usage";
  writeFileSync(
    join(appDir, "package.json"),
    `${JSON.stringify(runtimePackage, null, 2)}\n`,
  );
  copyFileSync(join(root, "apps/runtime/dsh-web.mjs"), join(appDir, "dsh-web.mjs"));
  cpSync(
    join(root, "packages/dsh-codex-usage"),
    join(appDir, "plugins/dsh-codex-usage"),
    { recursive: true },
  );

  // Do not inherit a developer machine's global `offline=true` setting. A
  // release build may reuse the local store, but it must still be allowed to
  // fetch any tarball that is not cached yet.
  const installArgs = [
    "install",
    "--ignore-workspace",
    "--node-linker=hoisted",
    "--offline=false",
  ];
  // Windows CreateProcess cannot execute pnpm.cmd directly. Use a constant
  // command string instead of shell:true, which is deprecated for argument
  // arrays and can change their escaping semantics.
  const install = process.platform === "win32"
    ? spawnSync(
        process.env.ComSpec || "cmd.exe",
        ["/d", "/s", "/c", `pnpm ${installArgs.join(" ")}`],
        { cwd: appDir, stdio: "inherit", shell: false },
      )
    : spawnSync("pnpm", installArgs, {
        cwd: appDir,
        stdio: "inherit",
        shell: false,
      });
  if (install.status !== 0) throw new Error(`pnpm install failed`);
  patchPackagedDshProfileBoot(appDir);
  console.log("closure ready");
}

// dsh-app-boot supports an installation-owned base URL specifically for
// closed packaged runtimes, but the current dsh CLI does not forward it. Give
// the Cordis loader a stable base inside the bundled closure so bare plugin
// imports do not depend on a persistent profile junction surviving upgrades.
// Keep this patch strict: a future dsh layout change must fail the build rather
// than silently reintroduce the installed-only startup failure.
function patchPackagedDshProfileBoot(appDir) {
  const dshLib = join(appDir, "node_modules", "@deepseek-ai", "dsh", "lib");
  const candidates = readdirSync(dshLib)
    .filter((name) => /^profile-boot-.*\.js$/.test(name))
    .map((name) => join(dshLib, name));
  const before = `\tconst ctx = await boot(NAME, rootConfig, structuredClone(allPatches(composed)), (hostCtx) => {
\t\tapp.current = hostCtx;
\t\thostCtx.provide(DSH_LAUNCH_ENVIRONMENT_KEY, options.environment);
\t\tprovideCmdline(hostCtx, {
\t\t\targs: options.args,
\t\t\texit: (code) => void shutdown.shutdown(code)
\t\t});
\t});`;
  const after = `\tconst ctx = await boot(NAME, rootConfig, structuredClone(allPatches(composed)), (hostCtx) => {
\t\tapp.current = hostCtx;
\t\thostCtx.provide(DSH_LAUNCH_ENVIRONMENT_KEY, options.environment);
\t\tprovideCmdline(hostCtx, {
\t\t\targs: options.args,
\t\t\texit: (code) => void shutdown.shutdown(code)
\t\t});
\t}, new URL("../", import.meta.url).href);`;
  const matches = [];
  for (const candidate of candidates) {
    const source = readFileSync(candidate, "utf8");
    if (source.includes(before)) matches.push({ candidate, source });
  }
  if (matches.length !== 1) {
    throw new Error(
      `expected exactly one dsh profile boot site, found ${matches.length}`,
    );
  }
  const [{ candidate, source }] = matches;
  writeFileSync(candidate, source.replace(before, after));
  console.log(`patched packaged dsh loader base: ${candidate}`);
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
