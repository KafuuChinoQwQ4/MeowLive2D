# application 目录索引

业务用例编排及外部能力接口定义

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
application/  # 业务用例编排及外部能力接口定义
├── src/  # Agent、事件调度、语音与资源任务用例
│   ├── agent/  # Agent 配置、输出校验、播放关联及状态类型
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── decisions.rs  # 模型输出校验、时效与繁忙复核、工作隔离和完整原文语音准备
│   │   ├── interaction.rs  # 有界流量统计、弹幕朗读模式、进房欢迎冷却及原文播报前缀
│   │   ├── playback.rs  # 播放状态同步和已完成对话记忆
│   │   ├── settings.rs  # 人设配置和调度资源上限校验
│   │   └── types.rs  # Agent 阶段、事件状态及应用调用结果
│   ├── ports/  # 业务方定义的模型、语音、存储和执行能力边界
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── companionship.rs  # 陪伴账本礼物确认与完成回执存储接口
│   │   ├── execution.rs  # Windows 执行指令下发、取消和执行回执接收的能力边界；实现不在业务层。
│   │   ├── live_source.rs  # 平台无关直播源、连接生命周期和错误语义接口
│   │   ├── llm.rs  # 模型决策、已完成对话与可取消异步模型接口
│   │   ├── llm_runtime.rs  # 统一模型工具轮次、流式观测与 token 用量接口
│   │   ├── memory.rs  # 记忆提取与向量嵌入能力接口
│   │   ├── memory_store.rs  # 权威记忆、后台任务及管理恢复存储接口
│   │   ├── mod.rs  # 由业务方定义的外部能力接口。实现位于 adapters 或应用入口的传输适配层。
│   │   ├── reasoning.rs  # 厂商无关的八档推理强度顺序、默认值与严格解析
│   │   ├── receipt_journal.rs  # 播放完成回执本地持久暂存与数据库提交确认接口
│   │   ├── relationships.rs  # 关系事实、图投影和同步恢复能力接口
│   │   ├── speech.rs  # 可动态注入的异步语音合成接口与 PCM 输出类型
│   │   ├── storage.rs  # 资源快照、参考音频与引擎路径存储接口
│   │   ├── training.rs  # 训练存储、同音色续训基底与受控进程接口
│   │   ├── viewer_merge.rs  # 身份合并预览及版本条件应用接口
│   │   ├── viewers.rs  # 观众身份和直播事件的幂等持久接收及分页查询端口
│   │   └── web_search.rs  # 只读网页搜索结果与异步查询业务接口
│   ├── scheduler/  # 候选事件优先级与礼物分组策略
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── fairness.rs  # 完成驱动的观众公平、有限重选和追问焦点
│   │   └── selection.rs  # SC 独立优先选择、欢迎单轮隔离、朗读长度约束与礼物分组
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── agent.rs  # Agent 生命周期、决策调度和状态快照
│   ├── lib.rs  # 业务用例与外部能力接口。通过注入 ports 的实现调用外部能力。
│   ├── performance.rs  # 协调发言、动作、下发与执行回执；处理代次、取消及重连后未知状态。
│   ├── resources.rs  # 角色与音色档案用例、删除和选择清理、持久化事务及映射一致性
│   ├── scheduler.rs  # 有界事件接收、去重、优先级与保留源事件剩余时效的调度
│   ├── session.rs  # 会话启动、暂停、恢复与关闭用例，协调在途任务的生命周期。
│   ├── speech.rs  # 单执行者语音 FIFO 队列、容量历史限制与取消回执状态机
│   └── training.rs  # 训练任务调度、同音色续训基底选择及版本保存选用
├── tests/  # 应用用例的队列与状态流转集成测试
│   ├── agent_support/  # Agent 测试公共夹具
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   └── mod.rs  # 事件、决策、会话和播放任务测试构造
│   ├── support/  # 应用层测试共用队列夹具
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   └── mod.rs  # 已连接队列、下发和完成流程测试夹具
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── agent_decisions.rs  # 输出校验、暂停隔离及决策冷却测试
│   ├── agent_fairness.rs  # 稳定身份公平调度与完成反馈边界测试
│   ├── agent_lifecycle.rs  # 配置、暂停、停止和播放生命周期测试
│   ├── agent_memory.rs  # 已完成对话数量及内容长度边界测试
│   ├── agent_settings.rs  # 人设配置与运行资源上限测试
│   ├── interaction_policy.rs  # SC 优先和时效、弹幕流量策略、欢迎抑制冷却及长原文播报测试
│   ├── resource_library.rs  # 资源事务失败保护、角色音色删除绑定及清理重试用例测试
│   ├── scheduler_bounds.rs  # 历史裁剪、去重淘汰、批次容量和克隆隔离测试
│   ├── scheduler_events.rs  # 事件去重、过期、容量及礼物分组测试
│   ├── speech_cancellation.rs  # 语音停止代次、迟到结果和断线取消测试
│   ├── speech_queue.rs  # 语音队列 FIFO、容量、接收门控和终态历史测试
│   └── speech_receipts.rs  # 执行回执顺序、重复回执与失败状态测试
├── Cargo.toml  # 该 Rust 包的名称、workspace 配置与模块依赖声明
└── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
```

可继续查看各子目录的索引：

- [src/](src/DIRECTORY.md)：Agent、事件调度、语音与资源任务用例
- [tests/](tests/DIRECTORY.md)：应用用例的队列与状态流转集成测试

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 9bd57394aacde56a75e35960097b114629f48441a2f19772facc14262dd7dfdf -->
