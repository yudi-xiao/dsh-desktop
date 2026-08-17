// ESLint flat config for the workspace TypeScript sources.
//
// Sources live in `apps/desktop/src` (the Tauri shell frontend) and
// `packages/plugin-market/src` (the shared UI library). Everything else —
// Rust, build output, generated icons, the bundled runtime — is ignored.
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: [
      "**/dist/**",
      "**/node_modules/**",
      "**/vendor/**",
      "**/target/**",
      "**/gen/**",
      "**/icons/**",
      "**/src-tauri/**",
    ],
  },
  ...tseslint.configs.recommended,
);
