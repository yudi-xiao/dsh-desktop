# dsh-desktop

把 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)（`dsh`）装进一个可安装、可扩展的跨平台桌面应用 —— 一个 Codex Desktop 的替代品。

## 架构（三层）

dsh 是纯 Node/TypeScript 的插件化 agent harness（pnpm monorepo + 一切皆插件），插件在运行时按字符串包名 `import()` 第三方 npm 包，**只能在 Node 上运行**；`dsh web` 通过本地 loopback HTTP 伺服 React Web UI 并注入 `__DSH_BOOT__` 引导。因此桌面应用由三层组成：

| 层 | 产物 | 职责 |
| --- | --- | --- |
| 壳 | `apps/desktop`（Tauri v2，Rust） | 原生窗口 + WebView；Node 后端进程守护（启动/就绪/退避重启/退出清理）；插件市场原生事务 |
| 后端 | `apps/runtime`（portable Node + `@deepseek-ai/dsh` closure） | 跑 dsh 的 cordis 插件树，HTTP 伺服前端与 API |
| 前端 | dsh Web UI（远程）+ 本地插件市场页 | 主窗口加载 dsh 的 loopback origin；插件市场独立窗口 |

关键取舍：

- **必须依赖 Node**：dsh 的 cordis 插件树（TS/ESM/npm 生态）只能在 Node 运行时上跑。桌面 app 不能假设系统装了 Node，因此打包 **portable Node 二进制 + `@deepseek-ai/dsh` closure（pnpm 部署后的 node_modules）**，经 Tauri resources 分发。**不使用 SEA 单文件可执行**：SEA 会把动态 `import()` 锁死在可执行内，破坏插件在运行时加载第三方 npm 包的能力。
- **必须走 HTTP**：`dsh web` 以 HTTP 伺服前端与 API，壳的 WebView 加载 `http://127.0.0.1:<port>`，端口由 OS 分配（`--port 0`）避免冲突。
- **壳必须存在**：Node 后端不提供原生窗口；壳承担窗口生命周期、后端守护，退出时终止后端进程组，不留孤儿 node。

## 目录结构

```
apps/
  desktop/          Tauri v2 壳（Rust src-tauri/ + React/TS/Vite 前端）
  runtime/          dsh Node 后端装配（pnpm 锁定 @deepseek-ai/dsh）
packages/
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
pnpm tauri build    # 打包当前平台
```

## 许可证

MIT
