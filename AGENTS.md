# dsh-desktop

把 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)（`dsh`）装进跨平台桌面应用 —— Codex Desktop 替代品。Tauri v2（Rust）壳 + 进程外 spawn 的 dsh Node 后端。仓库：https://github.com/yudi-xiao/dsh-desktop

## Project

- 四个运行模块：`apps/desktop`（Tauri v2 壳，Rust + React/TS/Vite）、`apps/runtime`（portable Node + `@deepseek-ai/dsh` closure）、`packages/plugin-market`（市场前端库）、`packages/dsh-codex-usage`（内置 dsh 用量插件）；另有 `scripts/` 负责构建编排
- 入口：`apps/desktop/src-tauri/src/main.rs` → `lib.rs::run()`；后端启动器 `apps/runtime/dsh-web.mjs`（spawn `dsh web`）
- 主窗口加载 dsh 的 loopback origin（connecting 占位页 → navigate）；插件市场（`market.html`）、项目看板（`board.html`）与后端设置（`settings.html`）为独立窗口，托盘菜单入口

## Commands

- `pnpm install` — 安装（根 `pnpm.onlyBuiltDependencies` 允许 native 构建：node-pty/koffi 等）
- `pnpm dev` — Tauri 开发窗口（dev 模式用 workspace 的 dsh-web.mjs，可热改）
- `pnpm dev:web` — 只启动 Vite 前端（不启动 Tauri 和 dsh 后端）
- `pnpm build` — 前端构建（tsc + vite 四入口 index/market/board/settings）
- `pnpm typecheck` — tsc --noEmit（desktop + plugin-market）
- `pnpm lint` — eslint flat config（扫描 `apps/desktop/src` + `packages/plugin-market/src`）
- `pnpm tauri build` — 打包当前平台；因 `createUpdaterArtifacts: true`，本地正式打包需先设置 `TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`（PowerShell 与 POSIX 示例见 `docs/RELEASING.md`）
- `cd apps/desktop/src-tauri && cargo test` — Rust 单测（supervisor/plugins/board/navigation）
- `node scripts/prepare-runtime.mjs` — 生成 portable Node + dsh closure（`vendor/`，被 gitignore）
- `node scripts/check-dsh-version.mjs` — 对比锁定 dsh 版本与 npm latest
- `node scripts/smoke-runtime.mjs` — 冒烟：dsh web 启动 → readiness → boot manifest/UI bundle/用量接口 HTTP 200 → 干净退出
- `pnpm smoke:codex:pinned` — 通过 pnpm dlx 启动固定版本 Codex app-server，验证 initialize + account/read；ChatGPT 已登录时再验证 rate limits/可选 usage（不登录、不启动 turn）

## Architecture

- `supervisor.rs`（HarnessSupervisor）：spawn `dsh web`、readiness 检测、崩溃退避重启、进程树清理（Windows taskkill /T、POSIX 进程组）
- `plugins.rs`：插件市场事务（隔离候选 Profile → 预览 diff → 应用备份 → 回滚），`dsh plugin` 走固定 DSH_HOME
- `packages/dsh-codex-usage`：内置 dsh 插件，提供「设置 → Codex」主控制面、用量展示与 Session Adapter；
  Tauri 仅暴露经 dsh loopback origin 校验的固定本机命令。托盘设置窗口只用于故障恢复
- `board.rs`：项目看板，读 `$DSH_HOME/storages/workspace.json`（dsh workspace 存储单元）投影为卡片（active/empty/archived 状态）
- `codex.rs`：检测系统或应用托管的 Codex CLI；精确锁定版本、spawn stdio app-server、ChatGPT managed OAuth、后端选择与 dsh session ↔ Codex thread 适配/持久化
- `codex_usage.rs`：归一化 rate limits/account usage/thread token usage，只落盘展示安全的 `codex-usage.json` 快照并发 Tauri 事件
- `packages/dsh-codex-usage`：dsh host 插件提供同源只读 JSON/SSE；client 插件注册 `sidebar.footer.action` + `settings.section`
- `lib.rs`：窗口创建、托盘（显示/看板/市场/后端设置/退出）、深链 `dsh://`、导航过滤、单实例聚焦；CSP 配置位于 `tauri.conf.json`
- 后端装配：portable Node 二进制 + hoisted closure 归档（`app.tar.gz`），经 Tauri resources 分发，运行时按 archive fingerprint 解压到 app 数据目录（避免升级继续复用旧 closure；归档规避 NSIS 长路径）

## Conventions

- **必须依赖 Node**：dsh 的 cordis 插件树（TS/ESM/npm）只能在 Node 跑；不用 SEA（会锁死运行时动态 `import()` 插件的能力）
- **DSH_HOME 固定**到 `app_data_dir/dsh`（supervisor 与 plugins/board 一致）
- **Windows 权限模式**：Windows 没有 dsh 的 bwrap/Landlock/Seatbelt confinement backend；未显式设置 `DSH_PERMISSION_MODE` 时，supervisor 会回退到 `danger-full-access` 并输出警告。需要审批/隔离语义时必须显式配置并验证运行环境
- **安装版日志**：每次启动写 `app_data_dir/logs/dsh-desktop-YYYY-MM-DD.log`，按自然日滚动并只保留 3 天；常见凭据字段整行脱敏。Windows 的 Rust 子进程必须经过 `platform::configure_background_command`，Node 嵌套 spawn 必须设置 `windowsHide: true`
- **Codex 版本策略**：Codex CLI 与 dsh 均无 LTS dist-tag；截至 2026-08-17 精确锁定最新非 alpha 版本 `@openai/codex@0.147.0` 与 `@deepseek-ai/dsh@0.1.0-rc.6`。Codex 不匹配时安装到 `app_data_dir/codex-cli/<version>`，不写全局 npm
- **Codex 认证**：优先使用 app-server `account/login/start` 的 `type: chatgpt` 浏览器 OAuth；由 Codex 持久化并刷新 token，不在桌面配置中保存 OpenAI API key
- **Codex 覆盖**：`DSH_DESKTOP_CODEX_CMD` 可覆盖 Codex 可执行文件，`DSH_DESKTOP_NPM_CMD` 可覆盖托管安装使用的 npm 命令
- **Codex UI 安全边界**：浏览器只读用量快照；session adapter 仅暴露带参数校验的 thread/turn/审批 commands，并校验主窗口 origin 必须等于 supervisor 当前 dsh origin；绝不暴露 OAuth token、API key 或通用 RPC
- **Codex 会话映射**：明确的 dsh session ↔ Codex thread 映射和精简消息历史写入 `app_data_dir/codex-sessions.json`；用量 remaining percent 仍仅为 `100 - usedPercent`
- **插件市场事务**：`web`（活动）→ `web-candidate`（隔离预览）→ `web-previous`（应用前备份）；undo 恢复 previous
- **导航过滤**：仅 `tauri` scheme + `127.0.0.1`/`localhost` 允许在窗口内加载，其余 http(s) 转系统浏览器
- **托盘常驻**：关闭窗口隐藏到托盘，仅托盘「退出」才终止后端进程树
- **深链**：`dsh://` 经 tauri-plugin-deep-link（single-instance 带 deep-link feature）注册 scheme 并转发 `deep-link` 事件
- **新增 Rust command**：在 `lib.rs` invoke_handler 注册；新增窗口需加入 `capabilities/default.json` 的 windows、vite 入口与对应 HTML
- **签名密钥**：`apps/desktop/.tauri-key`（私钥）绝提交，公钥写入 tauri.conf.json `plugins.updater.pubkey`；丢失密码需重生成密钥对并更新 pubkey

## TODO

- 看板「点击直达项目会话」：需 dsh 官方把 workspace 切换暴露为稳定 RPC/URL（当前 `/api/workspace/list` 返回 404；现仅「聚焦主窗口 + 打开目录」）
- 配置 GitHub secrets：`TAURI_SIGNING_PRIVATE_KEY` + `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`、Apple 签名/notarize 证书（`APPLE_*`）
- macOS/Windows 代码签名 + notarize（当前产物未签名，SmartScreen/Gatekeeper 会提示）
- CI（`build.yml` / `release.yml`）在真实 GitHub 上验证三平台构建与 release
- 打 tag（`v0.1.0`）触发正式发布

## Notes

- （后续补充）
