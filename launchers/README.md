# 启动入口

日常启动只需要这个文件夹。模型下载、服务连接和停止都在网页操作。

## Windows

1. 在实际克隆位置打开本文件夹；在 WSL 项目根目录运行 `explorer.exe ./launchers` 即可定位，无需固定发行版、用户名或绝对路径。
2. 双击 **start-windows.cmd**。程序检查 WSL2，然后打开控制面板。保持启动窗口打开。
3. 页面中先确认“环境与模型”，再到“启动与运行”依次打开 **主服务 → TTS 语音引擎 → Windows 执行端** 三个开关。
4. 首次先在“角色与音色”上传自己的参考录音，再在“训练与离线”导入素材并完成训练、试听和保存；选择自己的音色后即可播报。新检出不包含预置音色或训练结果。

源码仓库不包含 Windows 执行程序，首次需自行构建并准备配置，方法见 [根 README](../README.md)。准备后由第三个开关自动运行，默认读取项目 `target/windows-client/meowlive-client.exe` 与同目录的 `desktop.local.toml`，声音输出到 Windows 默认扬声器。

如果以前已经手动打开过执行端，先关闭原来的执行端窗口，再使用网页开关。页面会保护外部连接，不接管它。

退出时先停止播报，关闭 Windows 执行端、TTS 和主服务开关，再关闭启动窗口。关闭浏览器只隐藏界面，不等于停止程序。

## Linux / WSL

在项目根目录运行 `./launchers/start.sh`，或者 `npm start`。原生 Linux 可使用主服务和模型管理；Windows 执行端开关需要同一台 Windows 机器的 WSL2 环境。

## 文件说明

- `start-windows.cmd`：Windows 双击入口。
- `start-windows.ps1`：入口内部使用的 WSL2 检查和启动逻辑，不需要用户手动运行。
- `start.sh`：Linux / WSL 入口，自动切到项目目录后启动面板。
- `windows-client.ps1`：网页内部调用的 Windows 进程管理程序，不需要用户操作。

第一次安装 WSL2 时按启动器提示完成；开发依赖和构建步骤见 [根 README](../README.md)。Windows 开关不会编译缺失的执行程序；`target/windows-client/` 需有自行构建的 exe 和本机配置。该目录被 Git 忽略，不随仓库分发。
