# src 目录索引

播放、连接、口型、VTS、OBS 与模型导入的执行源码

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
src/  # 播放、连接、口型、VTS、OBS 与模型导入的执行源码
├── assets/  # Live2D 导出模型包的本地校验与安装
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── model.rs  # 模型清单与路径校验、无覆盖安装、模型身份枚举及持久化删除重试
├── audio/  # 音频设备后端、采样转换与设备播放时钟
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── buffer.rs  # 跨平台重采样输出缓冲，按需分配并限制实际排队 PCM 为 128 MiB，附长 SC 和溢出测试
│   ├── conversion.rs  # 相位连续的 PCM 重采样与声道映射
│   ├── meter.rs  # 固定容量的设备播放能量时间线与 RMS 累加器
│   ├── simulated.rs  # 显式静音模拟后端，按模拟播放时间退役样本并观测能量
│   ├── timing.rs  # 设备计划播放时间与完成回执时钟
│   └── windows.rs  # Windows 默认输出设备、受限按需重采样缓冲及真实输出能量和时钟回执
├── avatar/  # VTube Studio 私有协议、配置校验、授权存储与口型连接状态机
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── client.rs  # 有界 VTS WebSocket 请求响应、请求关联、API 错误和口型参数注入
│   ├── config.rs  # VTS 默认关闭配置与地址、参数名称、超时范围校验
│   ├── resources.rs  # VTS 已授权模型查询加载与热键核对预览
│   ├── switching.rs  # 切换角色口型输入并等待 VTS 应用确认
│   ├── token.rs  # 阻塞线程池中的私有授权令牌读写、权限校验及可取消等待
│   └── worker.rs  # 最新口型值消费、授权复用、归零、样本过期与自动重连状态机
├── bin/  # 独立桌面执行客户端命令入口
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── meowlive-client.rs  # 独立桌面执行客户端程序入口
│   └── meowlive-model.rs  # 独立模型包校验与显式目录安装命令行
├── lip_sync/  # 设备能量到口型的纯规则、配置与音频后端包装器
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── backend.rs  # 音频后端观察包装器，定时发布最新口型并在终态同步复位
│   ├── config.rs  # 口型阈值、增益、平滑时间和更新频率的默认值与校验
│   └── envelope.rs  # 按经过时间执行开闭口平滑与静音复位的纯计算器
├── obs/  # OBS 本地连接配置与校验
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── config.rs  # OBS 本机 WebSocket 连接、兼容环境变量和私有设置路径校验
│   └── settings.rs  # 执行端 OBS 设置私有文件读写、凭据保留清除及公开状态脱敏
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── assets.rs  # 本机 Live2D 模型包校验、安装、枚举及删除能力导出
├── audio.rs  # 音频设备边界、播放事件与设备输出能量观测接口
├── avatar.rs  # VTube Studio 独立连接入口、配置与可观察状态导出
├── cli.rs  # 桌面客户端配置加载、服务重连、VTS 组装与限时退出入口
├── config.rs  # 执行客户端连接、缓冲、VTS 与口型配置校验及路径解析
├── connection.rs  # 携带独立设备凭据连接主服务、接收控制音频及回传播放回执
├── host.rs  # Tauri 与 CLI 共用的可取消执行宿主及退出清理
├── lib.rs  # Windows 执行库：与 Tauri 和 React 解耦，生命周期由桌面进程管理。
├── lip_sync.rs  # 设备输出口型的配置、平滑器与观察后端导出
├── obs.rs  # 使用本机面板设置的 OBS WebSocket v5 鉴权、场景切换及录制控制
├── playback.rs  # 播放状态机、代次取消、乱序校验与设备回执
├── presentation.rs  # 桌面口型驱动生命周期与角色参数切换组装
└── resource_control.rs  # 桌面模型导入列举删除、VTS 加载热键与 OBS 控制执行
```

可继续查看各子目录的索引：

- [assets/](assets/DIRECTORY.md)：Live2D 导出模型包的本地校验与安装
- [audio/](audio/DIRECTORY.md)：音频设备后端、采样转换与设备播放时钟
- [avatar/](avatar/DIRECTORY.md)：VTube Studio 私有协议、配置校验、授权存储与口型连接状态机
- [bin/](bin/DIRECTORY.md)：独立桌面执行客户端命令入口
- [lip_sync/](lip_sync/DIRECTORY.md)：设备能量到口型的纯规则、配置与音频后端包装器
- [obs/](obs/DIRECTORY.md)：OBS 本地连接配置与校验

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 787ef80797e8161bc0c9c2a1da8b21ce9d1bdaa281413ff9d335807b715adbb4 -->
