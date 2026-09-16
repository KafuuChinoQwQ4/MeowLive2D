# tests 目录索引

应用用例的队列与状态流转集成测试

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
tests/  # 应用用例的队列与状态流转集成测试
├── agent_support/  # Agent 测试公共夹具
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── mod.rs  # 事件、决策、会话和播放任务测试构造
├── support/  # 应用层测试共用队列夹具
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── mod.rs  # 已连接队列、下发和完成流程测试夹具
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── agent_decisions.rs  # 输出校验、暂停隔离及决策冷却测试
├── agent_lifecycle.rs  # 配置、暂停、停止和播放生命周期测试
├── agent_memory.rs  # 已完成对话数量及内容长度边界测试
├── agent_settings.rs  # 人设配置与运行资源上限测试
├── resource_library.rs  # 资源保存一致性、容量与映射验证并发规则测试
├── scheduler_bounds.rs  # 历史裁剪、去重淘汰、批次容量和克隆隔离测试
├── scheduler_events.rs  # 事件去重、过期、容量及礼物分组测试
├── speech_cancellation.rs  # 语音停止代次、迟到结果和断线取消测试
├── speech_queue.rs  # 语音队列 FIFO、容量、接收门控和终态历史测试
└── speech_receipts.rs  # 执行回执顺序、重复回执与失败状态测试
```

可继续查看各子目录的索引：

- [agent_support/](agent_support/DIRECTORY.md)：Agent 测试公共夹具
- [support/](support/DIRECTORY.md)：应用层测试共用队列夹具

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: e69d08387e393c1906b91a22a6ac9add52e180930d3c691819acdd634d92b58f -->
