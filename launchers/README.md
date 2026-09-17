# 启动入口

日常启动只需要这个文件夹。模型下载、服务连接和停止都在网页操作。

## 第一次使用

当前提供源码，尚无安装包。先准备以下环境：

- Linux / WSL2：Rust 1.85+、Node.js 22.12+、npm 10+、Python 3.11+，以及 GPT-SoVITS 引擎和对应的 Python 环境。
- Windows：VTube Studio，以及下面说明的 Windows 执行程序。需要推流时再安装 OBS。

在 Linux / WSL 终端下载项目：

```bash
git clone https://github.com/KafuuChinoQwQ4/MeowLive2D.git
cd MeowLive2D
./launchers/start.sh
```

首次启动会安装网页依赖并生成 `config/local/launcher.json`。其中 `ttsPython` 填 GPT-SoVITS 的 Python 路径，`ttsEngineRoot` 填引擎目录；具体写法见 [配置说明](../config/README.md)。网页可以下载语音模型，但不会自动安装语音引擎及其依赖。

### 准备 Windows 执行程序

在安装了 Rust MSVC 工具链的 Windows 环境中，进入一份项目源码目录并执行：

```powershell
cargo build -p meowlive-desktop-runtime --bin meowlive-client --release --locked
```

将生成的 `target/release/meowlive-client.exe` 复制到日常使用的 WSL 项目的 `target/windows-client/`。再把 [desktop.example.toml](../config/desktop.example.toml) 复制到同一目录，命名为 `desktop.local.toml`；按实际网络设置 `server_url`，使 Windows 能连接主服务。

准备好后，日常启动不需要在 Windows 编译代码或安装 Node.js。程序和本机配置均不随仓库分发。

## Windows

1. 在实际克隆位置打开本文件夹；在 WSL 项目根目录运行 `explorer.exe ./launchers` 即可定位，无需固定发行版、用户名或绝对路径。
2. 双击 **start-windows.cmd**。程序检查 WSL2，然后打开控制面板。保持启动窗口打开。
3. 页面中先确认“环境与模型”，再到“启动与运行”依次打开 **主服务 → TTS 语音引擎 → Windows 执行端** 三个开关。受管 TTS 启动后模型保持关闭，需到“训练与离线”点击“启用语音模型”才能播报或试听；关闭模型可释放权重内存，TTS 服务仍可保留。
4. 在“角色与音色”上传自己的参考录音，填写对应文字并设为当前音色，然后去“语音播报”试播。训练是可选步骤，新检出不包含预置音色或训练结果。

第三个开关会运行上面准备好的 `target/windows-client/meowlive-client.exe`，并读取同目录的 `desktop.local.toml`，声音输出到 Windows 默认扬声器。

如果以前已经手动打开过执行端，先关闭原来的执行端窗口，再使用网页开关。页面会保护外部连接，不接管它。

退出时先停止播报，关闭 Windows 执行端、TTS 和主服务开关，再关闭启动窗口。关闭浏览器只隐藏界面，不等于停止程序。

## Linux / WSL

在项目根目录运行 `./launchers/start.sh`，或者 `npm start`。原生 Linux 可使用主服务和模型管理；Windows 执行端开关需要同一台 Windows 机器的 WSL2 环境。

停止时按 Ctrl+C，并等待清理完成、终端返回提示符。若原启动窗口已消失，网页仍提示“已由其他终端启动”，在 WSL 的项目根目录运行 `npm run stop`，完成后再 `npm start`。清理命令只处理当前检出的已识别服务，超时会强制回收 Linux 服务进程组，并向本项目受管 Windows 执行端发送停止请求。

## 文件说明

- `start-windows.cmd`：Windows 双击入口。
- `start-windows.ps1`：入口内部使用的 WSL2 检查和启动逻辑，不需要用户手动运行。
- `start.sh`：Linux / WSL 入口，自动切到项目目录后启动面板。
- `windows-client.ps1`：网页内部调用的 Windows 进程管理程序，不需要用户操作。

第一次安装 WSL2 时按启动器提示完成；Windows 开关不会编译缺失的执行程序，请先完成本页“第一次使用”的准备。
