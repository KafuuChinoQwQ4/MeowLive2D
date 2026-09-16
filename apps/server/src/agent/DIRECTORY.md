# agent 目录索引

Agent 服务状态、跨端映射和异步模型调度

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
agent/  # Agent 服务状态、跨端映射和异步模型调度
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── mapping.rs  # 统一事件及 Agent 业务状态到公开 HTTP DTO 的映射
├── state.rs  # Agent 查询控制、原子事件接收及语音状态同步
└── worker.rs  # 锁外模型调用、有界重试、GPU 在途互斥及取消核对后的语音入队
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 5c16426c025a19fc1e49784a101cbf0b94a39174f705c3fa4b3e5024e314e11c -->
