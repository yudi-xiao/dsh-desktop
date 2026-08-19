# dsh-desktop

把 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)（`dsh`）装进一个可安装、可扩展的跨平台桌面应用 —— 一个 Codex Desktop 的替代品。

> 仓库：https://github.com/yudi-xiao/dsh-desktop
>
> ![CI](https://github.com/yudi-xiao/dsh-desktop/actions/workflows/build.yml/badge.svg)

## 安装

从 [Releases](https://github.com/yudi-xiao/dsh-desktop/releases/latest) 下载对应平台的安装包（Windows `.exe` / macOS `.dmg` / Linux `.AppImage`·`.deb`）。应用内置 portable Node 与 dsh closure，无需预装 Node。

安装版每次启动都会写运行日志。Windows 日志位于
`%APPDATA%\com.dshdesktop.client\logs\dsh-desktop-YYYY-MM-DD.log`；其他平台位于
系统应用数据目录的 `com.dshdesktop.client/logs/`。日志按自然日滚动，仅保留当天和前两天；
常见 Authorization、OAuth token 与 API key 字段会被整行脱敏。Windows 后台启动的 dsh、
Codex、npm 和插件命令均不会创建可见的 CLI 窗口。

## 架构（三层）

dsh 是纯 Node/TypeScript 的插件化 agent harness（pnpm monorepo + 一切皆插件），插件在运行时按字符串包名 `import()` 第三方 npm 包，**只能在 Node 上运行**；`dsh web` 通过本地 loopback HTTP 伺服 React Web UI 并注入 `__DSH_BOOT__` 引导。因此桌面应用由三层组成：

| 层 | 产物 | 职责 |
| --- | --- | --- |
| 壳 | `apps/desktop`（Tauri v2，Rust） | 原生窗口 + WebView；Node 后端进程守护；插件市场事务；Codex CLI/app-server 配置与生命周期 |
| 后端 | `apps/runtime`（portable Node + `@deepseek-ai/dsh` closure） | 跑 dsh 的 cordis 插件树，HTTP 伺服前端与 API |
| 前端 | dsh Web UI（远程）+ 本地插件市场页 | 主窗口加载 dsh 的 loopback origin；插件市场独立窗口 |

关键取舍：

- **必须依赖 Node**：dsh 的 cordis 插件树（TS/ESM/npm 生态）只能在 Node 运行时上跑。桌面 app 不能假设系统装了 Node，因此打包 **portable Node 二进制 + `@deepseek-ai/dsh` closure（pnpm 部署后的 node_modules）**，经 Tauri resources 分发。**不使用 SEA 单文件可执行**：SEA 会把动态 `import()` 锁死在可执行内，破坏插件在运行时加载第三方 npm 包的能力。
- **必须走 HTTP**：`dsh web` 以 HTTP 伺服前端与 API，壳的 WebView 加载 `http://127.0.0.1:<port>`，端口由 OS 分配（`--port 0`）避免冲突。
- **壳必须存在**：Node 后端不提供原生窗口；壳承担窗口生命周期、后端守护，退出时终止后端进程组，不留孤儿 node。
- **runtime 按内容代际解压**：`app.tar.gz` 的大小与修改时间组成 extraction fingerprint；同一归档复用，应用升级后的新归档进入新目录，避免继续运行旧 closure。

## 目录结构

```
apps/
  desktop/          Tauri v2 壳（Rust src-tauri/ + React/TS/Vite 前端）
  runtime/          dsh Node 后端装配（pnpm 锁定 @deepseek-ai/dsh）
packages/
  dsh-codex-usage/  内置 dsh 插件（Codex 会话交接 + 状态/订阅/Token 用量 UI）
  plugin-market/    插件市场前端库（React 组件 + 契约类型）
scripts/            构建编排（portable Node、closure 装配、图标、打包）
```

## 插件

- 第三方插件仍由 dsh 官方 Profile / Loader 管理，走 `$DSH_HOME/profiles/<name>/node_modules` 动态加载。
- 桌面层提供可视化插件市场：浏览/搜索/安装/隔离预览/应用/回滚，事务封装在 Rust command 中（参考 oh-dsh marketplace）。

## 开发

依赖：Node `^22.19 || >=24`、pnpm 10、Rust stable。

```sh
pnpm install
pnpm dev            # 启动 Tauri 开发窗口（spawn dsh web 子进程）
pnpm dev:web        # 只启动 Vite 前端
pnpm tauri build    # 打包当前平台（正式包需要 updater 签名私钥）
```

运行后直接在 dsh Web UI 打开「设置 → Codex」。选择 Codex 时，内置 dsh 插件会通过受限的
桌面桥接检测
`@openai/codex@0.147.0`；没有精确匹配版本时安装到应用数据目录，随后启动
stdio app-server，并优先使用 ChatGPT managed OAuth。详见
[`docs/CODEX_BACKEND.md`](docs/CODEX_BACKEND.md)。

托盘菜单中的「后端设置（故障恢复）」不是常规入口，只在 dsh Web UI 无法进入时用于恢复。

启用后，dsh 侧栏底部会显示 Codex 当前限额窗口的近似剩余比例；完整的 CLI/app-server
状态、ChatGPT 账户与套餐、多限额桶、重置时间、Credits、可选 Token 历史和最近线程用量位于
「设置 → Codex」。同一页面还可切换 dsh/Codex、检测或托管安装 CLI、登录/退出 ChatGPT、刷新
app-server 状态。浏览器端只调用来源受限的明确命令并读取清洗后的本地快照，不接触 OAuth token、
API key 或 app-server 原始 RPC。

dsh 会话标题栏同时提供「交给 Codex」：首次发送会将当前工作区、最近对话上下文和当前任务
映射到一个本地 Codex thread，后续消息继续该 thread，并在抽屉中流式显示回复、命令/文件进度、
停止与用户审批。会话桥只接受 supervisor 当前 dsh origin 的受限命令，不向 Web UI 开放通用 RPC。

## 许可证

MIT
