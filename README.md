# MeowLive2D

让 Live2D 角色用自定义音色说话，由 AI 回应 B 站直播间的弹幕、礼物和 SC。支持人物卡、音色训练、观众记录与 OBS 控制。

## Windows 安装版

从 [Releases](https://github.com/KafuuChinoQwQ4/MeowLive2D/releases) 下载预览安装包，安装后打开 App。主服务和桌面执行端随 App 启动，无需安装 Node.js 或 Rust。

在版本说明顶部按平台点击“下载安装包”；名称含“更新资源”的技术发布供 App 自动增量更新使用，无需手动下载。当前提供 Windows x64，其他平台的安装包将在支持后出现在下载表中。

**安装包不会自动补齐全部环境：**

- 语音播报：自行准备 TTS 引擎和模型。
- Live2D 形象：自行安装 VTube Studio，加载模型并开启插件 API。
- 观众档案、积分和记忆：准备 PostgreSQL + pgvector，按[数据库启用步骤](config/windows-database.md)配置。首次默认关闭观众存储。
- 本地声音训练：在 App「环境与模型」检测或安装 WSL2 语音后端，下载并选用 GPT-SoVITS v2；训练需要可用 CUDA GPU。
- 推流与录制：按需安装 OBS。

原生 App 基础功能不需要 WSL。声音环境向导可在用户点击后安装 WSL2 和独立语音后端；系统授权和重启由用户完成。桌面配置与打包见 [Windows App 说明](apps/desktop/src-tauri/README.md)。

## 开始使用

1. **准备声音**：配置 TTS，使用已适配的 GPT-SoVITS v2 模型。在「声音训练」上传 3–10 秒参考录音，填写原文并选用音色；不必先训练。
2. **试播**：确认服务就绪，在「语音播报」输入文字并加入队列。
3. **设置角色**：连接 VTube Studio，在「角色与人物卡」选择形象并填写人设。嘴型输入使用 `MeowMouthOpen`，范围 `0–1`。
4. **连接 AI**：在「LLM 接入」填写地址和密钥，获取模型、测试并保存，然后重启 App 或启动器。
5. **开始互动**：在「Agent 互动」保存设置并恢复 Agent，先用模拟事件试用。
6. **接入直播**：在「直播连接」填写 B 站开放平台凭据和主播身份码，连接直播间；仅填房间号不能接入。

App「环境与模型」也提供 GitHub 版本检查与签名更新；首次或无缓存时完整下载，后续复用已验证分块。发布配置见 [更新发布说明](releases/UPDATING.md)。

各页面的具体操作见应用内「使用指南」。异常先查「运行日志」，Agent 执行过程查「Agent 观察」。

## 源码运行

先按[启动说明](launchers/README.md)准备 Linux / WSL2、构建工具、语音引擎和 Windows 执行程序，再在项目根目录运行：

```bash
./launchers/start.sh
```

Windows 也可双击 `launchers/start-windows.cmd`。打开控制面板 <http://127.0.0.1:1420>，保持启动终端开启。源码启动器可自动准备项目 PostgreSQL，但需要 Docker 已运行。

退出时在启动终端按 `Ctrl+C`；关闭浏览器不会停止服务。Windows 安装版直接关闭 App。

## 更多说明

- [服务与模型配置](config/README.md)
- [Agent 工具与用量](config/agent-runtime.md)
- [数据库备份与恢复](config/viewer-recovery.md)
- [Windows 播放与直播实机验证](tests/e2e/README.md)

## 开发

需要 Rust 1.85+、Node.js 22.12+、npm 10+。

```bash
npm ci
npm run check
npm run build
```

推送或提交 PR 后，GitHub Actions 自动执行检查并构建 Windows 安装包，可在 CI 的 `windows-installer` 附件下载。发布时同步版本号，添加 `releases/标签名.md`，再推送对应标签（`v版本号` 或 `v版本号-windows-preview.YYYYMMDD`）；全部检查通过后自动发布 Windows 安装包；正式版本标签发布为 Release，带日期的预览标签发布为预览版。Dependabot 定期提交依赖更新 PR，不自动合并。

主服务在 `apps/server`，前端在 `apps/desktop`。依赖方向为 `application → domain`；外部适配器实现 application 接口，跨端契约集中在 protocol，Windows 执行库独立于 Tauri 外壳。前端通过 services 访问外部能力。

文件用途见[目录索引](DIRECTORY.md)，协作规则见 [AGENTS.md](AGENTS.md)。修改后运行 `npm run tree:update` 和 `npm run tree:check`。私有配置、密钥、模型与运行数据不随源码提交。

源码采用 [MIT 许可证](LICENSE)，第三方模型、引擎和素材遵循各自授权。
