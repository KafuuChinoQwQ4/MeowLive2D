# protocol 目录索引

跨进程控制、事件、音频和执行消息的契约源

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
protocol/  # 跨进程控制、事件、音频和执行消息的契约源
├── src/  # 与业务领域分离的通信 DTO 模块
│   ├── bin/  # 协议开发命令入口
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   └── export-types.rs  # 从 Rust DTO 生成并检查含 Agent 观察模型的 TypeScript 契约
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── agent.rs  # Agent 设置、直播事件、阶段、处理状态与公开快照 DTO
│   ├── agent_observability.rs  # Agent 调度快照、Trace、Turn、步骤及列表跨端契约
│   ├── audio.rs  # PCM 格式约定及有界二进制音频帧编解码
│   ├── auth.rs  # 管理员认证状态、登录请求和短期会话响应契约
│   ├── companionship.rs  # 仅供管理端的陪伴账本与礼物确认契约
│   ├── control.rs  # 文字播报请求、状态快照与桌面控制消息契约
│   ├── event.rs  # 提供给控制面板的状态通知和事件封装；不透传平台私有事件。
│   ├── execution.rs  # 桌面执行状态与播放回执契约
│   ├── launcher.rs  # 本机主服务、TTS 和 Windows 执行端三开关的启动管理契约
│   ├── lib.rs  # 跨进程通信契约的唯一来源。与业务领域对象分离，按协议版本演进。
│   ├── live.rs  # 直播平台连接状态、面板凭据配置请求与脱敏快照契约
│   ├── llm.rs  # LLM 接入配置、模型目录与统一推理档位预览的跨端契约
│   ├── llm_runtime.rs  # 运行配置、模型单价、Trace Turn 关联用量与活动跨端契约
│   ├── memory.rs  # 记忆证据管理和后台任务状态跨端契约
│   ├── model_library.rs  # 本机环境、模型目录、安装结果及下载任务的跨进程契约
│   ├── obs.rs  # OBS 场景录制操作、本机连接设置与脱敏状态契约
│   ├── relationships.rs  # 关系事实管理和图同步状态跨端契约
│   ├── resources.rs  # 角色音色档案及桌面模型、OBS 操作和私有设置桥接契约
│   ├── training.rs  # 训练片段、性能参数、任务版本及离线测量跨端契约
│   ├── training_runtime.rs  # 独立于 TTS 服务的模型内存启停请求与状态契约
│   ├── viewer_merge.rs  # 身份合并预览和明确确认的跨端契约
│   └── viewers.rs  # 管理员观众身份、昵称历史和持久事件分页契约
├── tests/  # 通信协议独立集成测试
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── agent_contracts.rs  # Agent 输入与观察契约的严格反序列化及公开 JSON 形状测试
│   ├── audio_frames.rs  # PCM 二进制帧编码、边界与损坏输入测试
│   ├── compatibility.rs  # 协议必填字段、训练模式缺省兼容、未知标签与音频格式测试
│   ├── control_serialization.rs  # 控制消息与执行回执序列化测试
│   └── obs_contracts.rs  # OBS 指令及设置请求未知字段拒绝、凭据脱敏与桌面资源契约测试
├── Cargo.toml  # 该 Rust 包的名称、workspace 配置与模块依赖声明
└── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
```

可继续查看各子目录的索引：

- [src/](src/DIRECTORY.md)：与业务领域分离的通信 DTO 模块
- [tests/](tests/DIRECTORY.md)：通信协议独立集成测试

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 6622f322d2f3e358a5184c7c2a58c9079a071085d4a2b4eba31ed692ac186b70 -->
