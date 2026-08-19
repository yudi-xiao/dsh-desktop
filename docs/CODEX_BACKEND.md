# Codex 本地后端

DSH Desktop 可以保留 dsh Web UI，同时启动本机 Codex app-server 作为可选会话后端。
主控制面由内置 dsh 插件提供：打开 dsh Web UI 的「设置 → Codex」，用户选择 Codex 后执行以下流程：

1. 检查已保存路径、应用托管路径和系统 `PATH` 中的 Codex CLI。
2. 仅接受精确版本 `0.147.0`；缺失或版本不匹配时执行应用内托管安装。
3. 启动 `codex app-server --listen stdio://`。
4. 发送 `initialize` 和 `initialized` 握手。
5. 调用 `account/read` 检查登录状态。
6. 非 ChatGPT managed 登录时调用 `account/login/start`，使用 `type: chatgpt`，
   并在系统浏览器打开 app-server 返回的 `authUrl`。
7. 监听 `account/login/completed` 与 `account/updated`，登录成功后进入 ready 状态。

切换 dsh/Codex、检测与托管安装 CLI、启动/修复 app-server、ChatGPT 登录/退出、状态与用量刷新
均在该插件页面完成。托盘「后端设置（故障恢复）」只保留为 dsh Web UI 无法进入时的恢复入口，
不再是正常配置路径。

官方协议说明：[Codex app-server](https://developers.openai.com/codex/app-server)。

## 版本策略

2026-08-17 查询 npm dist-tags 的结果：

- `@openai/codex`：`latest = 0.147.0`，另有 `alpha`，没有 `lts`。
- `@deepseek-ai/dsh`：`latest = 0.1.0-rc.6`，没有 `lts`。

因此项目锁定上述精确的最新非 alpha 版本，而不是跟随浮动 `latest`。升级时应同时：

1. 查询 npm dist-tags。
2. 修改 `codex.rs` 的 `CODEX_CLI_VERSION` 或 `apps/runtime/package.json` 的 dsh 版本。
3. 为目标 Codex 版本运行 `codex app-server generate-json-schema` 或 `generate-ts`，检查协议差异。
4. 验证检测、安装、initialize、account/read、OAuth 和 thread/turn RPC。

只读验证固定版本的 app-server 握手和账号状态接口：

```sh
pnpm smoke:codex:pinned
```

## 安装位置

托管 Codex 安装在：

```text
<app_data_dir>/codex-cli/0.147.0/
```

等价 npm 命令为：

```sh
npm install --global --prefix <managed-prefix> @openai/codex@0.147.0 --no-audit --no-fund
```

打包环境优先使用 portable Node 自带的 npm CLI；开发环境回退到 `PATH` 中的 npm。
不会执行管理员级全局安装，也不会覆盖用户已有的 Codex CLI。

## 状态和接口

Tauri commands：

- `codex_status`：检测 CLI 并返回当前状态。
- `backend_select`：选择 `dsh` 或 `codex`。
- `codex_configure`：检测/安装、启动 app-server 并检查认证。
- `codex_login_chatgpt`：重新发起 ChatGPT managed OAuth。
- `codex_logout_chatgpt`：调用 app-server `account/logout` 退出当前 ChatGPT 账户。
- `codex_session_poll`：读取当前 dsh session 对应的 Codex thread、消息和增量事件。
- `codex_collaboration_mode_catalog`：读取 app-server 的原生标准/计划模式，并限制为客户端支持的模式。
- `codex_session_send`：首次交接时创建 thread，后续调用 `turn/start` 继续同一 thread；通过
  `turn/start.collaborationMode` 原生选择标准模式或计划模式。
- `codex_session_goal_get` / `codex_session_goal_update` / `codex_session_goal_clear`：读取、设定、
  暂停/恢复或清除当前 thread 的持久目标。
- `codex_session_interrupt`：中止当前 Codex turn。
- `codex_session_approve`：只响应当前会话的命令/文件修改审批。
- `codex_session_reset`：解除 dsh session 映射，让下一次发送创建新的 Codex thread。
- `codex_usage_status`：返回经过清洗的账户/订阅/线程用量快照。
- `codex_usage_refresh`：后台重新读取 app-server 的账户用量。

Tauri events：

- `codex-status`：安装、启动、认证状态快照。
- `codex-app-server-event`：app-server 原始通知，仅供 Tauri 内部窗口集成。
- `codex-session-updated`：经过会话路由后的更新提示；dsh Web UI 同时使用受限 command 轮询，
  不接触 app-server stdio。
- `codex-usage-updated`：只含可展示字段的用量快照。

后端选择保存到 `<app_data_dir>/backend.json`。只保存后端、CLI 路径和版本，
不保存 OAuth token 或 API key；认证数据完全由 Codex 管理。

## dsh UI 用量面板

`codex_usage.rs` 订阅和读取以下 app-server 数据：

- `account/rateLimits/read` 与 `account/rateLimits/updated`：一个或多个限额桶、
  `usedPercent`、窗口时长和 Unix 重置时间。
- `account/usage/read`：累计/单日 Token 和连续使用统计。此方法在当前官方文档中存在，
  但固定版 `0.147.0` 的生成 Schema 尚未声明，因此按可选能力调用；method-not-found 时隐藏统计卡，
  不影响限额展示。
- `thread/tokenUsage/updated`：按 Codex thread ID 保存最近一轮、累计 Token 和模型上下文窗口。

Rust 只把可展示字段写入 `<app_data_dir>/codex-usage.json`，并把路径通过
`DSH_DESKTOP_CODEX_USAGE_FILE` 传给 dsh。内置包
`packages/dsh-codex-usage` 将其暴露为仅同源、只读接口：

- `GET /desktop-api/codex/usage`：当前快照，`Cache-Control: no-store`。
- `GET /desktop-api/codex/usage/events`：SSE 增量快照。

浏览器看不到 app-server stdio、OAuth token、API key、审批内容或通用 RPC。插件注册
`sidebar.footer.action` 和 `settings.section`：侧栏显示紧凑剩余比例，设置页既是 Codex 的主控制面，
也展示 CLI/app-server、账户/套餐、限额窗口、Credits、可选使用历史和最近 Codex 线程。
控制命令只接受内置恢复窗口或 supervisor 当前 dsh loopback origin，其他 Web origin 会被拒绝。

“剩余比例”由 `100 - usedPercent` 推导，是当前窗口的近似比例，不是剩余 Token 的绝对数量。
用量面板仍按 Codex thread ID 展示；会话适配器另外把 dsh session ID 与 thread ID 的明确映射
保存到 `<app_data_dir>/codex-sessions.json`，不会根据时间或标题猜测绑定。

## dsh session adapter

dsh 对话标题栏提供「交给 Codex」入口。首次发送会把当前工作区、最近的 dsh 用户/助手消息和
当前请求发送给 `thread/start` + `turn/start`；后续发送恢复并继续同一个 Codex thread。抽屉消费：

- `item/agentMessage/delta`：流式助手文本；
- `item/started` / `item/completed`：命令、文件修改和最终消息；
- `turn/plan/updated` / `item/plan/delta`：计划步骤与计划正文流式展示；
- `turn/diff/updated`：工作区变更提示；
- `turn/completed`：成功、失败、中止收口；
- `item/commandExecution/requestApproval` / `item/fileChange/requestApproval`：用户审批。

只有主窗口且当前 URL origin 与 supervisor 实际启动的 dsh origin 完全一致时，session commands
才会执行。浏览器不能指定任意 app-server method；不支持的 server request 返回协议错误并安全终止，
不会悬挂 Codex turn。映射和最多 200 条精简用户/助手消息会持久化，OAuth token、API key、完整
工具输出和审批内容不会写入该文件。

模式选择器复用 dsh 原「标准模式」控件的位置、28px 高度、圆角与交互层级；切换 Codex 后显示
「标准模式 / 计划模式」，切回 dsh 时插件撤出并恢复原界面。目标条位于会话和输入框之间，沿用 dsh
目标插件的 dock 形态。模式、目标和计划消息均跟随明确的 dsh session ↔ Codex thread 映射持久化，
不依赖提示词模拟。

## 环境覆盖

- `DSH_DESKTOP_CODEX_CMD`：指定自定义 Codex 可执行文件。
- `DSH_DESKTOP_NPM_CMD`：指定应用内安装使用的 npm 命令。
- `DSH_DESKTOP_CODEX_USAGE_FILE`：由 supervisor 设置的清洗快照路径；不应指向凭证文件。

Codex app-server 当前仍属于实验性接口。发布构建应保持精确版本和协议兼容测试，
不要把未经认证的 WebSocket listener 暴露到 loopback 以外；桌面集成默认只使用 stdio。
