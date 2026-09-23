# agent_observability 目录索引

Agent Trace 内存状态机、持久恢复与有界保留实现

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
agent_observability/  # Agent Trace 内存状态机、持久恢复与有界保留实现
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── mod.rs  # Agent 调度快照、Trace Turn 状态机、语音关联和降级历史
└── persistence.rs  # Agent 活动 Trace 原子保存、完成分段、恢复与保留策略
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 0df8aac6a46aab613eb496493a4c356258e22eb1da1344ce463fda7915d56a22 -->
