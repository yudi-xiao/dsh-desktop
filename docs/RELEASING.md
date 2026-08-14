# 发布、签名与自动更新

## 触发发布

推一个 `v*` 标签（如 `v0.1.0`）会触发 `.github/workflows/release.yml`，在
macOS / Windows / Linux 矩阵上构建安装包并创建 GitHub Release（draft）。

```sh
git tag v0.1.0
git push origin v0.1.0
```

## 代码签名与公证（可选，未配置时产物未签名）

签名证书由 GitHub Actions secrets 提供；缺失时 `tauri-action` 仍会产出未签名
安装包（macOS Gatekeeper / Windows SmartScreen 会提示）。

### macOS（签名 + notarize）

在仓库 secrets 中配置：

- `APPLE_CERTIFICATE` — base64 的 `.p12` 开发者证书
- `APPLE_CERTIFICATE_PASSWORD` — 证书密码
- `APPLE_SIGNING_IDENTITY` — 签名身份（如 `Developer ID Application: …`）
- `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` — notarization 凭据

### Windows（代码签名）

- `TAURI_SIGNING_PRIVATE_KEY` — Tauri updater 签名私钥（`tauri signer generate`）
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — 私钥密码

## 自动更新

更新使用 `tauri-plugin-updater`。签名密钥对用以下命令生成：

```sh
pnpm --filter @dsh-desktop/desktop tauri signer generate -w ./.tauri-key
```

- **公钥**写入 `apps/desktop/src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`。
- **私钥**（`.tauri-key`）绝不提交，作为 `TAURI_SIGNING_PRIVATE_KEY` secret 供 CI
  签名更新包使用；私钥密码作为 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。

更新端点 `plugins.updater.endpoints` 指向发布 host 的 `latest.json`（当前指向
本仓库的 GitHub Releases）。发布新版本时，用
[tauri-cli 的 updater 支持](https://v2.tauri.app/plugin/updater/) 生成
`latest.json` 并上传到 release assets。

> 若丢失私钥或密码，将无法为更新包签名，自动更新失效。
