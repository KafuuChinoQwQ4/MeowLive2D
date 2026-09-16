# src 目录索引

Agent、事件调度、语音与资源任务用例

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
src/  # Agent、事件调度、语音与资源任务用例
├── agent/  # Agent 配置、输出校验、播放关联及状态类型
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── decisions.rs  # 模型输出校验、工作编号隔离和语音准备
│   ├── playback.rs  # 播放状态同步和已完成对话记忆
│   ├── settings.rs  # 人设配置和调度资源上限校验
│   └── types.rs  # Agent 阶段、事件状态及应用调用结果
├── ports/  # 业务方定义的模型、语音、存储和执行能力边界
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── execution.rs  # Windows 执行指令下发、取消和执行回执接收的能力边界；实现不在业务层。
│   ├── live_source.rs  # 平台无关直播源、连接生命周期和错误语义接口
│   ├── llm.rs  # 模型决策、已完成对话与可取消异步模型接口
│   ├── mod.rs  # 由业务方定义的外部能力接口。实现位于 adapters 或应用入口的传输适配层。
│   ├── speech.rs  # 可动态注入的异步语音合成接口与 PCM 输出类型
│   ├── storage.rs  # 资源快照、参考音频与引擎路径存储接口
│   └── training.rs  # 训练素材存储、成对模型解析与受控进程执行接口
├── scheduler/  # 候选事件优先级与礼物分组策略
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── selection.rs  # 礼物优先选择和有界原始事件分组
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── agent.rs  # Agent 生命周期、决策调度和状态快照
├── lib.rs  # 业务用例与外部能力接口。通过注入 ports 的实现调用外部能力。
├── performance.rs  # 协调发言、动作、下发与执行回执；处理代次、取消及重连后未知状态。
├── resources.rs  # 角色与音色档案用例、持久化事务和映射验证一致性
├── scheduler.rs  # 有界事件存储、独立去重和终态历史裁剪
├── session.rs  # 会话启动、暂停、恢复与关闭用例，协调在途任务的生命周期。
├── speech.rs  # 单执行者语音 FIFO 队列、容量历史限制与取消回执状态机
└── training.rs  # 单任务训练生命周期、失败恢复、取消、试听、音色保存与版本启用用例
```

可继续查看各子目录的索引：

- [agent/](agent/DIRECTORY.md)：Agent 配置、输出校验、播放关联及状态类型
- [ports/](ports/DIRECTORY.md)：业务方定义的模型、语音、存储和执行能力边界
- [scheduler/](scheduler/DIRECTORY.md)：候选事件优先级与礼物分组策略

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 441cab9a8bd50b05a10fa1affb583bc6f1bcbd5b5b686f0d95ef2068b6db0ccc -->
