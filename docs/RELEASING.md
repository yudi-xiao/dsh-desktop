# 发布、签名与自动更新

## 触发发布

推一个 `v*` 标签（如 `v0.1.0`）会触发 `.github/workflows/release.yml`，在
macOS / Windows / Linux 矩阵上构建安装包并创建 GitHub Release（draft）。

```sh
git tag v0.1.0
git push origin v0.1.0
```

## 平台代码签名与公证

平台签名证书由 GitHub Actions secrets 提供。macOS 证书缺失时会产出未签名
安装包，Gatekeeper 会提示。Windows Authenticode 代码签名当前尚未配置，
`TAURI_SIGNING_PRIVATE_KEY` 仅用于 Tauri updater 产物签名，不能消除 SmartScreen 提示。

### macOS（签名 + notarize）

在仓库 secrets 中配置：

- `APPLE_CERTIFICATE` — base64 的 `.p12` 开发者证书
- `APPLE_CERTIFICATE_PASSWORD` — 证书密码
- `APPLE_SIGNING_IDENTITY` — 签名身份（如 `Developer ID Application: …`）
- `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` — notarization 凭据

### Tauri updater 产物签名（正式发布必须）

- `TAURI_SIGNING_PRIVATE_KEY` — Tauri updater 签名私钥（`tauri signer generate`，用于所有发布平台）
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — 私钥密码

## 自动更新

更新使用 `tauri-plugin-updater`。签名密钥对用以下命令生成：

```sh
pnpm --filter @dsh-desktop/desktop tauri signer generate -w ./.tauri-key
```

- **公钥**写入 `apps/desktop/src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`。
- **私钥**（`.tauri-key`）绝不提交，作为 `TAURI_SIGNING_PRIVATE_KEY` secret 供 CI
  签名更新包使用；私钥密码作为 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。

`tauri.conf.json` 里 `bundle.createUpdaterArtifacts: true` 让 Tauri 在构建时生成
`.sig` 签名文件；`tauri-action` 据此自动生成 `latest.json` 并上传到 release
assets（`includeUpdaterJson`/`uploadUpdaterJson` 默认开启）。二者缺一不可：
没有 `createUpdaterArtifacts` 就不会有 `.sig`，`latest.json` 也就不会生成。

更新端点 `plugins.updater.endpoints` 指向
`https://github.com/<owner>/dsh-desktop/releases/latest/download/latest.json`。

### 本地正式构建需要签名私钥

因为 `createUpdaterArtifacts: true`，本地 `pnpm tauri build` 也必须提供私钥，
否则报错 `A public key has been found, but no private key`：

POSIX shell：

```sh
export TAURI_SIGNING_PRIVATE_KEY="$(cat apps/desktop/.tauri-key)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="<密码>"
pnpm tauri build
```

Windows PowerShell：

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -LiteralPath apps/desktop/.tauri-key -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "<密码>"
pnpm tauri build
```

普通分支和 Pull Request 的 CI 使用 `tauri.ci.conf.json` 关闭 updater 产物，
因此不需要向非发布构建暴露签名私钥。

> 若丢失私钥或密码，将无法为更新包签名，自动更新失效，需重新生成密钥对
> 并更新 `pubkey`。
