// Reports the pinned @deepseek-ai/dsh version vs the npm latest, so upgrades
// can be scheduled deliberately (dsh is a developer preview with breaking
// changes; see docs/RELEASING.md).
//
//   node scripts/check-dsh-version.mjs
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");

const runtimePkg = JSON.parse(
  readFileSync(join(root, "apps/runtime/package.json"), "utf8"),
);
const pinned = runtimePkg.dependencies["@deepseek-ai/dsh"];

let latest;
try {
  const res = await fetch("https://registry.npmjs.org/@deepseek-ai/dsh/latest");
  latest = (await res.json()).version;
} catch {
  console.log(`pinned: ${pinned}\nlatest: unknown (offline?)`);
  process.exit(0);
}

console.log(`pinned @deepseek-ai/dsh: ${pinned}`);
console.log(`npm latest:             ${latest}`);
if (pinned === latest) {
  console.log("up to date");
} else {
  console.log(
    `⚠️  upgrade available: ${pinned} → ${latest}. dsh is a developer preview —`,
    "check the upstream release notes for breaking changes before bumping",
    "apps/runtime/package.json and re-running scripts/prepare-runtime.mjs.",
  );
}
