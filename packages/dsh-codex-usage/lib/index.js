import { watch } from "node:fs";
import { readFile } from "node:fs/promises";
import { basename, dirname } from "node:path";

export const name = "desktop-codex-usage";
export const inject = ["webServer"];

const JSON_ENDPOINT = "/desktop-api/codex/usage";
const EVENTS_ENDPOINT = "/desktop-api/codex/usage/events";

function unavailableSnapshot(error) {
  return {
    backend: "dsh",
    phase: "idle",
    cliVersion: null,
    requiredCliVersion: "0.147.0",
    appServerRunning: false,
    authMode: null,
    managedInstall: false,
    state: "unavailable",
    planType: null,
    email: null,
    updatedAt: Date.now(),
    buckets: [],
    credits: null,
    resetCredits: null,
    accountUsage: null,
    threadUsage: {},
    error,
  };
}

async function readSnapshot(file) {
  if (!file) return unavailableSnapshot("Codex usage bridge is not configured");
  try {
    const value = JSON.parse(await readFile(file, "utf8"));
    return value && typeof value === "object"
      ? value
      : unavailableSnapshot("Codex usage snapshot is invalid");
  } catch {
    // Do not reflect filesystem paths or platform error details into the
    // browser origin.
    return unavailableSnapshot("Codex usage snapshot is unavailable");
  }
}

function sendJson(res, status, value, headOnly = false) {
  const body = JSON.stringify(value);
  res.writeHead(status, {
    "Content-Type": "application/json; charset=utf-8",
    "Cache-Control": "no-store",
    "X-Content-Type-Options": "nosniff",
    "Content-Length": Buffer.byteLength(body),
  });
  res.end(headOnly ? undefined : body);
}

export function apply(ctx) {
  const file = process.env.DSH_DESKTOP_CODEX_USAGE_FILE;
  let current = unavailableSnapshot(null);
  let serialized = JSON.stringify(current);
  let reading = false;
  const clients = new Set();

  const broadcast = (next) => {
    current = next;
    serialized = JSON.stringify(next);
    for (const res of clients) res.write(`data: ${serialized}\n\n`);
  };

  const refresh = async () => {
    if (reading) return;
    reading = true;
    try {
      const next = await readSnapshot(file);
      const nextSerialized = JSON.stringify(next);
      if (nextSerialized !== serialized) broadcast(next);
    } finally {
      reading = false;
    }
  };

  ctx.effect(() => {
    void refresh();
    const disposeJson = ctx.webServer.register({
      kind: "exact",
      path: JSON_ENDPOINT,
      handler: async (req, res) => {
        if (req.method !== "GET" && req.method !== "HEAD") {
          res.writeHead(405, { Allow: "GET, HEAD" });
          res.end();
          return;
        }
        await refresh();
        sendJson(res, 200, current, req.method === "HEAD");
      },
    });
    const disposeEvents = ctx.webServer.register({
      kind: "exact",
      path: EVENTS_ENDPOINT,
      handler: (req, res) => {
        if (req.method !== "GET") {
          res.writeHead(405, { Allow: "GET" });
          res.end();
          return;
        }
        res.writeHead(200, {
          "Content-Type": "text/event-stream; charset=utf-8",
          "Cache-Control": "no-store",
          Connection: "keep-alive",
          "X-Content-Type-Options": "nosniff",
        });
        res.write(": connected\n\n");
        res.write(`data: ${serialized}\n\n`);
        clients.add(res);
        res.on("close", () => clients.delete(res));
      },
    });

    let watcher;
    if (file) {
      try {
        const target = basename(file);
        watcher = watch(dirname(file), (_event, changed) => {
          if (!changed || changed.toString() === target) void refresh();
        });
      } catch (error) {
        console.warn("[desktop-codex-usage] snapshot watch unavailable:", error);
      }
    }
    const poll = setInterval(() => void refresh(), 5000);
    const heartbeat = setInterval(() => {
      for (const res of clients) res.write(": heartbeat\n\n");
    }, 20000);

    return () => {
      disposeJson();
      disposeEvents();
      watcher?.close();
      clearInterval(poll);
      clearInterval(heartbeat);
      for (const res of clients) res.end();
      clients.clear();
    };
  }, "desktop-codex-usage: same-origin snapshot bridge");
}
