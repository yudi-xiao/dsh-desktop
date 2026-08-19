#!/usr/bin/env node
// Reuse one DSH_HOME across two complete boots. This catches upgrade-only
// failures involving persistent profile junctions and the embedded desktop
// plugin that a clean-profile smoke test cannot reproduce.
import { spawn } from "node:child_process";
import {
  existsSync,
  linkSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  realpathSync,
  renameSync,
  rmdirSync,
  symlinkSync,
  unlinkSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const smoke = join(here, "smoke-runtime.mjs");
const dshHome = mkdtempSync(join(tmpdir(), "dsh-desktop-upgrade-smoke-"));
const launcher = process.env.DSH_DESKTOP_SMOKE_LAUNCHER ||
  join(here, "..", "apps", "runtime", "dsh-web.mjs");
let expectedPackageSource;
let expectedDanglingPackageSource;

function hardlinkPackageFiles(source, target) {
  mkdirSync(target, { recursive: true });
  for (const entry of readdirSync(source, { withFileTypes: true })) {
    if (entry.name === "node_modules") continue;
    const from = join(source, entry.name);
    const to = join(target, entry.name);
    if (entry.isDirectory()) hardlinkPackageFiles(from, to);
    else if (entry.isFile()) {
      if (existsSync(to)) unlinkSync(to);
      linkSync(from, to);
    }
  }
}

function reproducePnpmHardlinks() {
  const source = realpathSync(join(
    dirname(launcher),
    "node_modules",
    "@dsh-desktop",
    "dsh-codex-usage",
  ));
  const target = join(
    dshHome,
    "node_modules",
    "@dsh-desktop",
    "dsh-codex-usage",
  );
  hardlinkPackageFiles(source, target);
  const fallback = join(dshHome, "profiles", "node_modules");
  const staleFallback = join(dshHome, "profiles", "node_modules.before-upgrade");
  if (existsSync(fallback)) renameSync(fallback, staleFallback);
  console.log("upgrade smoke: reproduced pnpm hard-linked desktop plugin");
  console.log("upgrade smoke: moved the generated fallback aside to reproduce a legacy profile");
}

function reproduceLiveStaleRuntimeLink() {
  const packageName = "dsh-client-ui-plan";
  const target = join(
    dshHome,
    "profiles",
    "web",
    "node_modules",
    "@deepseek-ai",
    packageName,
  );
  const staleSource = join(
    dshHome,
    "runtime.before-upgrade",
    "node_modules",
    "@deepseek-ai",
    packageName,
  );
  mkdirSync(staleSource, { recursive: true });
  const metadata = lstatSync(target);
  if (!metadata.isSymbolicLink()) {
    throw new Error(`expected application-owned package link: ${target}`);
  }
  expectedPackageSource = realpathSync(target);
  if (process.platform === "win32") rmdirSync(target);
  else unlinkSync(target);
  symlinkSync(
    staleSource,
    target,
    process.platform === "win32" ? "junction" : "dir",
  );
  console.log("upgrade smoke: injected a live link to the previous runtime");
}

function reproduceDanglingStaleRuntimeLink() {
  const packageParts = ["@anthropic-ai", "sdk"];
  const target = join(
    dshHome,
    "profiles",
    "node_modules",
    ...packageParts,
  );
  const previousTarget = join(
    dshHome,
    "profiles",
    "node_modules.before-upgrade",
    ...packageParts,
  );
  const metadata = lstatSync(previousTarget);
  if (!metadata.isSymbolicLink()) {
    throw new Error(`expected application-owned package link: ${previousTarget}`);
  }
  expectedDanglingPackageSource = realpathSync(previousTarget);
  mkdirSync(dirname(target), { recursive: true });
  symlinkSync(
    join(dshHome, "runtime.removed", "node_modules", "@anthropic-ai", "sdk"),
    target,
    process.platform === "win32" ? "junction" : "dir",
  );
  console.log("upgrade smoke: injected the installed-client dangling @anthropic-ai/sdk link");
}

function assertProfileLocalFallback() {
  const localPackage = join(
    dshHome,
    "profiles",
    "web",
    "node_modules",
    "@deepseek-ai",
    "dsh-client-ui-plan",
    "package.json",
  );
  if (!existsSync(localPackage)) {
    throw new Error(`profile-local module fallback was not created: ${localPackage}`);
  }
  console.log("upgrade smoke: profile-local module fallback is available");
}

function assertProfileLinkReconciled() {
  const packageName = "dsh-client-ui-plan";
  const target = join(
    dshHome,
    "profiles",
    "web",
    "node_modules",
    "@deepseek-ai",
    packageName,
  );
  if (!expectedPackageSource) {
    throw new Error("upgrade smoke did not capture the bundled package source");
  }
  const expected = expectedPackageSource;
  const actual = realpathSync(target);
  const normalize = (value) => process.platform === "win32"
    ? value.toLowerCase()
    : value;
  if (normalize(actual) !== normalize(expected)) {
    throw new Error(`stale runtime link was not reconciled: ${actual}`);
  }
  console.log("upgrade smoke: stale runtime link was reconciled");
}

function assertDanglingProfileLinkReconciled() {
  const target = join(
    dshHome,
    "profiles",
    "node_modules",
    "@anthropic-ai",
    "sdk",
  );
  if (!expectedDanglingPackageSource) {
    throw new Error("upgrade smoke did not capture the bundled @anthropic-ai/sdk source");
  }
  const actual = realpathSync(target);
  const normalize = (value) => process.platform === "win32"
    ? value.toLowerCase()
    : value;
  if (normalize(actual) !== normalize(expectedDanglingPackageSource)) {
    throw new Error(`dangling runtime link was not reconciled: ${actual}`);
  }
  console.log("upgrade smoke: dangling @anthropic-ai/sdk link was reconciled");
}

for (let attempt = 1; attempt <= 2; attempt += 1) {
  console.log(`upgrade smoke boot ${attempt}/2: ${dshHome}`);
  const code = await new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [smoke], {
      stdio: "inherit",
      env: {
        ...process.env,
        DSH_DESKTOP_SMOKE_HOME: dshHome,
      },
      windowsHide: true,
    });
    child.once("error", reject);
    child.once("exit", (value) => resolve(value ?? 1));
  });
  if (code !== 0) process.exit(code);
  assertProfileLocalFallback();
  if (attempt === 1) {
    reproducePnpmHardlinks();
    reproduceLiveStaleRuntimeLink();
    reproduceDanglingStaleRuntimeLink();
  } else {
    assertProfileLinkReconciled();
    assertDanglingProfileLinkReconciled();
  }
}

console.log("upgrade smoke: persistent profile survived two boots");
