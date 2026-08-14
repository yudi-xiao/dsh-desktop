# scripts/

构建编排脚本（portable Node 下载、`@deepseek-ai/dsh` closure 装配、图标生成、三平台打包）。

| 脚本 | 用途 |
| --- | --- |
| `prepare-runtime.mjs` | 下载 portable Node + hoisted 安装 dsh closure + 归档 `app.tar.gz` 到 `vendor/runtime/<target>/` |
| `smoke-runtime.mjs` | 启动 `dsh web` → 等待 readiness → 校验 HTTP 200 → 干净退出（升级后/CI 冒烟） |
| `check-dsh-version.mjs` | 对比 pinned 与 npm latest 的 `@deepseek-ai/dsh` 版本 |
| `gen-icon.mjs` | 从源图生成占位应用图标（`tauri icon` 前体） |

## 升级 dsh

1. `node scripts/check-dsh-version.mjs` 查看可升级版本。
2. 修改 `apps/runtime/package.json` 的 `@deepseek-ai/dsh` 为精确版本。
3. `pnpm install` 更新 lockfile。
4. `node scripts/prepare-runtime.mjs` 重新装配 runtime。
5. `node scripts/smoke-runtime.mjs` 冒烟验证（dsh 是开发者预览，升级可能引入
   breaking change，冒烟失败时回退版本）。
6. 三平台 CI 构建安装包（`git tag v*` 触发 release）。

## 三平台打包

各平台需在对应 OS 上执行（dsh 有架构相关 native 模块）：

- Windows：`pnpm --filter @dsh-desktop/desktop tauri build`
- macOS：在 macOS 上执行（`aarch64` / `x86_64` 分别构建）
- Linux：在 Linux 上执行

详见 `docs/RELEASING.md`。
