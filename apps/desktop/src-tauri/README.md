# Windows App

安装 Windows x64 预览版后，打开 MeowLive2D 即可使用控制面板。App 自动启动内置主服务和音频执行端；页面展示启动进度、错误和日志位置，没有主服务手动开关。正常关闭窗口或 App 异常退出后，受管主服务通过父进程管道关闭并清理任务。已有兼容主服务会被复用，关闭 App 不会结束外部服务。

首次启动在 `%APPDATA%\io.meowlive.desktop\` 创建 `desktop.toml`、`server/server.toml` 和 `server.log`，资源与本机设置也保存在此目录。已有配置不会覆盖。需要重启生效的设置保存后，关闭并重新打开 App 即可。端口冲突和启动失败会显示在运行总览，处理后重新打开 App。

预览包包含主服务和桌面执行端，不包含 TTS 引擎、模型、VTube Studio、OBS 或 PostgreSQL。TTS 默认连接 `http://127.0.0.1:9880`；可在 `server/server.toml` 的 `[speech]` 配置中更改。首次配置关闭观众档案持久化；在 App 的 **环境与模型 → 数据库与可选功能** 打开官方下载来源，按 [Windows 数据库补齐步骤](../../../config/windows-database.md) 准备 PostgreSQL + pgvector 并启用 `[viewers]`；Linux / WSL 源码配置不受影响。在 App **环境与模型 → 声音环境** 检测或安装 WSL2、选择发行版、安装独立 GPT-SoVITS 后端，下载并选用模型后点击“连接训练后端”。Windows 主服务保存原有配置、人物卡及音色数据，训练和语音推理由 WSL 执行；无需切换主服务地址。训练需要 CUDA GPU，CPU 可用于推理。安装 WSL 可能要求管理员授权和手动重启，重启后重新检测继续。真实模型、直播和音视频联动仍需实机验收。

`desktop.toml` 支持配置其他主服务地址；远程地址仅连接、不在本机拉起服务。模型目录和 VTS token 路径相对于配置文件目录解析。VTS 使用前须在桌面配置启用 `[vtube_studio] enabled = true`，在 VTube Studio 开启插件 API 并允许授权。`--simulate` 用于无设备声音的协议验证。

## 构建

在 Windows 安装 Rust MSVC、Node.js 和 WebView2；Linux 交叉编译另需 cargo-xwin、LLVM（clang-cl、lld-link、llvm-rc）、NSIS 和 Rust 的 `x86_64-pc-windows-msvc` target。在项目根目录运行：

```sh
npm ci
npm run desktop:build
```

脚本先构建配套主服务，再构建前端和 Tauri，使用静态 MSVC CRT，安装器输出到 `target/x86_64-pc-windows-msvc/release/bundle/nsis/`。安装时若缺少 WebView2 会下载引导程序。安装包未配置代码签名。打包使用 Tauri 官方支持的 [cargo-xwin / NSIS 交叉编译流程](https://v2.tauri.app/distribute/windows-installer/)。

开发窗口使用 `npm run desktop:dev`。首次需先构建配套主服务，确保 `meowlive-server.exe` 位于桌面 EXE 同目录。也可连接已启动的外部服务。指定独立配置的示例：

```powershell
meowlive-desktop.exe --config C:\MeowLive2D\desktop.toml --simulate
```

显式配置下，受管服务数据写入该配置所在目录，便于隔离验收。独立 CLI 的 `--once` 不适用于 Tauri 外壳。

Tauri 只负责窗口、配置、主服务生命周期和 `desktop-runtime::host::RuntimeHandle`；播放、VTS、OBS、模型文件操作仍在独立执行库。前端通过 `services/desktop` 读取有界 IPC 状态，等待主服务健康检查通过才启用业务页面。IPC 不暴露密钥。窗口/CSP/能力声明见 `tauri.conf.json` 和 `capabilities/default.json`；`gen/` 和 `target/` 是排除索引的生成目录。

## App 内更新

同一“环境与模型”页面提供 GitHub Releases 更新检查、版本说明、下载和“安装并重启”。签名验证通过后才执行安装；环境任务、训练和直播期间不能安装更新。更新保留用户数据和模型。首次没有缓存时完整下载，后续按内容分块复用，页面显示真实下载量与复用量；无法提供分块时回退完整安装包。发布者须配置签名密钥，步骤与旧版本迁移见 [更新发布说明](../../../releases/UPDATING.md)。

WSL 环境选择记录保存在 `desktop-environment.json`，语音后端独立安装在所选发行版的 `/opt/meowlive-voice`。切换模型前先停止语音引擎，选择后点击“连接训练后端”应用配置；现有配置会备份为 `server/server.toml.before-voice`。已有 GPT-SoVITS 环境可直接复用：重新检测会读取 WSL 用户目录下 `code/MeowLive2D/config/server.local.toml` 或 `MeowLive2D/config/server.local.toml`，识别其中的 `[training] python` 和 `engine_root`；其他位置可展开“使用已有训练环境”填写配置文件的 WSL 绝对路径。检测成功后选用 GPT-SoVITS v2 并连接后端。已安装完整的模型会复用，缺失模型下载到专用后端目录，不覆盖原引擎；运行语音引擎期间请先停止再更换环境。App 不删除用户已有 WSL 发行版。环境操作进行中关闭窗口会被阻止，应先取消或等待完成。
