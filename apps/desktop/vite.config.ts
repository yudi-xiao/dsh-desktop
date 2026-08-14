import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath, URL } from "node:url";

// Tauri v2 recommends a fixed dev port so the CLI can reach the dev server.
// The production build is served from the local dist (src-tauri bundle) or,
// for the main window, from the dsh web child process's loopback origin.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  resolve: {
    alias: {
      // Compile the workspace plugin-market package from source.
      "@dsh-desktop/plugin-market": fileURLToPath(
        new URL("../../packages/plugin-market/src/index.tsx", import.meta.url),
      ),
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    target: "esnext",
    sourcemap: false,
    rollupOptions: {
      input: {
        index: fileURLToPath(new URL("index.html", import.meta.url)),
        market: fileURLToPath(new URL("market.html", import.meta.url)),
      },
    },
  },
});

