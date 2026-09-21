# scheduler 目录索引

候选事件优先级与礼物分组策略

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
scheduler/  # 候选事件优先级与礼物分组策略
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── fairness.rs  # 完成驱动的观众公平、有限重选和追问焦点
└── selection.rs  # SC 独立优先选择、欢迎单轮隔离、朗读长度约束与礼物分组
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 71e037d61668ea65de7a40dd617e08dfb67cde36605c191f1545dc16f0bde65b -->
