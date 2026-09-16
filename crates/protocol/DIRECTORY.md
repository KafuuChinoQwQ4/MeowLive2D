# protocol 目录索引

跨进程控制、事件、音频和执行消息的契约源

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
protocol/  # 跨进程控制、事件、音频和执行消息的契约源
├── src/  # 与业务领域分离的通信 DTO 模块
│   ├── bin/  # 协议开发命令入口
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   └── export-types.rs  # 从 Rust DTO 生成并检查 TypeScript 契约
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── agent.rs  # Agent 设置、直播事件、阶段、处理状态与公开快照 DTO
│   ├── audio.rs  # PCM 格式约定及有界二进制音频帧编解码
│   ├── control.rs  # 文字播报请求、状态快照与桌面控制消息契约
│   ├── event.rs  # 提供给控制面板的状态通知和事件封装；不透传平台私有事件。
│   ├── execution.rs  # 桌面执行状态与播放回执契约
│   ├── launcher.rs  # 本机主服务、TTS 和 Windows 执行端三开关的启动管理契约
│   ├── lib.rs  # 跨进程通信契约的唯一来源。与业务领域对象分离，按协议版本演进。
│   ├── live.rs  # 直播连接阶段、公开状态与诊断计数的跨端契约
│   ├── model_library.rs  # 本机环境、模型目录、安装结果及下载任务的跨进程契约
│   ├── obs.rs  # OBS 状态、场景与录制操作公开契约及严格反序列化
│   ├── resources.rs  # 跨端角色音色档案及桌面资源操作契约
│   └── training.rs  # 训练任务、模型版本与保存状态、试听和本地资源测量跨端契约
├── tests/  # 通信协议独立集成测试
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── agent_contracts.rs  # Agent 输入严格反序列化及公开 JSON 形状测试
│   ├── audio_frames.rs  # PCM 二进制帧编码、边界与损坏输入测试
│   ├── compatibility.rs  # 协议必填字段、标签与音频格式兼容测试
│   ├── control_serialization.rs  # 控制消息与执行回执序列化测试
│   └── obs_contracts.rs  # OBS 指令未知字段拒绝与资源通道契约测试
├── Cargo.toml  # 该 Rust 包的名称、workspace 配置与模块依赖声明
└── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
```

可继续查看各子目录的索引：

- [src/](src/DIRECTORY.md)：与业务领域分离的通信 DTO 模块
- [tests/](tests/DIRECTORY.md)：通信协议独立集成测试

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: a10a812257b51e768360bd8c60f7b682f440e28aaf5d51b9c777ef947ad09f29 -->
