# agent 目录索引

Agent 服务状态、跨端映射和异步模型调度

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
agent/  # Agent 服务状态、跨端映射和异步模型调度
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── mapping.rs  # 统一事件及 Agent 业务状态到公开 HTTP DTO 的映射
├── state.rs  # Agent 查询控制、本机设置保存、原子事件接收及语音状态同步
└── worker.rs  # 实时 Agent 调度、资料期限版本栅栏及持久回应关联
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: e4585ce9d97b9b6d4bd3b315241106f2d48e49a82758a312811d867b50de3c2e -->
