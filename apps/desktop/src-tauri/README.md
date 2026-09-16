# 可选 Windows 桌面外壳

默认开发方式是 Linux 网页加 Windows 独立执行端，无需运行本目录外壳。只有需要原生窗口时才使用以下流程。

Tauri 2 只创建面板窗口、读取用户配置和管理 `desktop-runtime::host::RuntimeHandle`。播放、VTS、OBS、模型文件操作仍在独立执行库；Linux 可运行 CLI 模拟后端，不启动 Tauri 窗口。

在 Windows 准备 Rust MSVC 工具链、Node.js 与 WebView2，项目根目录执行：

```powershell
npm ci
npm run desktop:dev
# 明确指定配置文件，使用绝对路径避免 Tauri 工作目录差异
npm run desktop:dev -- -- -- --config C:\MeowLive2D\config\desktop.local.toml
```

主服务与 GPT-SoVITS 必须分别启动。未指定 `--config` 时，在 Tauri 应用配置目录创建 `desktop.toml`，通常为 `%APPDATA%\io.meowlive.desktop\desktop.toml`；已有文件保持原样。模型目录、VTS token 路径相对于配置文件所在目录解析。面板先通过 `desktop_status` 读取主服务地址和宿主状态，再创建 HTTP 客户端；读取失败显示错误和重试，不连到默认地址。

默认使用 Windows 系统输出设备，`--simulate` 仅用于静音协议验证。关闭窗口取消执行循环、清理播放并关闭 VTS 驱动。前端仅消费必要状态，不能读取 OBS 密码、VTS token 或模型提供商密钥。独立 CLI 支持 `--once`，Tauri 外壳不支持这个选项。服务端允许来源需包括开发地址与 `http://tauri.localhost`。

窗口/CSP/能力声明在 `tauri.conf.json` 和 `capabilities/default.json`。应用图标沿用面板绿色 M，SVG 为可编辑源，PNG 与 ICO 为窗口构建输入。`custom-protocol` 用于 Tauri 正式构建，默认仅开发模式。自动生成的 `gen/` 与 `target/` 均排除 Git 和目录索引。

本轮不运行 `desktop:build`，不生成安装包。Linux 验证覆盖配置初始化、相对路径、参数校验、可取消宿主和 IPC 读取；不等于 Windows Tauri 编译、窗口、音频设备及 VTS/OBS 实机通过。完整实机步骤见项目 [E2E 验收](../../../tests/e2e/README.md)。
