# dsh-desktop

把 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)（`dsh`）装进跨平台桌面应用 —— Codex Desktop 替代品。Tauri v2（Rust）壳 + 进程外 spawn 的 dsh Node 后端。仓库：https://github.com/yudi-xiao/dsh-desktop

## Project

- 三层：`apps/desktop`（Tauri v2 壳，Rust + React/TS/Vite）、`apps/runtime`（portable Node + `@deepseek-ai/dsh` closure）、`packages/plugin-market`（市场前端库）、`scripts/`（构建编排）
- 入口：`apps/desktop/src-tauri/src/main.rs` → `lib.rs::run()`；后端启动器 `apps/runtime/dsh-web.mjs`（spawn `dsh web`）
- 主窗口加载 dsh 的 loopback origin（connecting 占位页 → navigate）；插件市场（`market.html`）与项目看板（`board.html`）为独立窗口，托盘菜单入口

## Commands

- `pnpm install` — 安装（根 `pnpm.onlyBuiltDependencies` 允许 native 构建：node-pty/koffi 等）
- `pnpm dev` — Tauri 开发窗口（dev 模式用 workspace 的 dsh-web.mjs，可热改）
- `pnpm build` — 前端构建（tsc + vite 三入口 index/market/board）
- `pnpm typecheck` — tsc --noEmit（desktop + plugin-market）
- `pnpm lint` — eslint flat config（扫描 `apps/desktop/src` + `packages/plugin-market/src`）
- `pnpm tauri build` — 打包当前平台；因 `createUpdaterArtifacts: true` 需先 `export TAURI_SIGNING_PRIVATE_KEY="$(cat apps/desktop/.tauri-key)"` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- `cd apps/desktop/src-tauri && cargo test` — Rust 单测（supervisor/plugins/board/navigation）
- `node scripts/prepare-runtime.mjs` — 生成 portable Node + dsh closure（`vendor/`，被 gitignore）
- `node scripts/check-dsh-version.mjs` — 对比锁定 dsh 版本与 npm latest
- `node scripts/smoke-runtime.mjs` — 冒烟：dsh web 启动 → readiness → HTTP 200 → 干净退出

## Architecture

- `supervisor.rs`（HarnessSupervisor）：spawn `dsh web`、readiness 检测、崩溃退避重启、进程树清理（Windows taskkill /T、POSIX 进程组）、单实例
- `plugins.rs`：插件市场事务（隔离候选 Profile → 预览 diff → 应用备份 → 回滚），`dsh plugin` 走固定 DSH_HOME
- `board.rs`：项目看板，读 `$DSH_HOME/storages/workspace.json`（dsh workspace 存储单元）投影为卡片（active/empty/archived 状态）
- `lib.rs`：窗口创建、托盘（显示/看板/市场/退出）、深链 `dsh://`、导航过滤、CSP、单实例聚焦
- 后端装配：portable Node 二进制 + hoisted closure 归档（`app.tar.gz`），经 Tauri resources 分发，运行时解压到 app 数据目录（归档规避 NSIS 长路径）

## Conventions

- **必须依赖 Node**：dsh 的 cordis 插件树（TS/ESM/npm）只能在 Node 跑；不用 SEA（会锁死运行时动态 `import()` 插件的能力）
- **DSH_HOME 固定**到 `app_data_dir/dsh`（supervisor 与 plugins/board 一致）
- **插件市场事务**：`web`（活动）→ `web-candidate`（隔离预览）→ `web-previous`（应用前备份）；undo 恢复 previous
- **导航过滤**：仅 `tauri` scheme + `127.0.0.1`/`localhost` 允许在窗口内加载，其余 http(s) 转系统浏览器
- **托盘常驻**：关闭窗口隐藏到托盘，仅托盘「退出」才终止后端进程树
- **深链**：`dsh://` 经 tauri-plugin-deep-link（single-instance 带 deep-link feature）注册 scheme 并转发 `deep-link` 事件
- **新增 Rust command**：在 `lib.rs` invoke_handler 注册；新增窗口需加入 `capabilities/default.json` 的 windows 与 vite 入口
- **签名密钥**：`apps/desktop/.tauri-key`（私钥）绝提交，公钥写入 tauri.conf.json `plugins.updater.pubkey`；丢失密码需重生成密钥对并更新 pubkey

## TODO

- 看板「点击直达项目会话」：需 dsh 官方把 workspace 切换暴露为稳定 RPC/URL（当前 `/api/workspace/list` 返回 404；现仅「聚焦主窗口 + 打开目录」）
- 配置 GitHub secrets：`TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`、Apple 签名/notarize 证书（`APPLE_*`）
- macOS/Windows 代码签名 + notarize（当前产物未签名，SmartScreen/Gatekeeper 会提示）
- CI（`build.yml` / `release.yml`）在真实 GitHub 上验证三平台构建与 release
- 打 tag（`v0.1.0`）触发正式发布

## Notes

- （后续补充）
