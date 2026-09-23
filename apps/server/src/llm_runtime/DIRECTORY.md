# llm_runtime 目录索引

运行配置、计量与持久化实现

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
llm_runtime/  # 运行配置、计量与持久化实现
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── ledger.rs  # 历史用量筛选聚合与缓存子集费用估算
├── metering.rs  # 单次模型调用计量、Trace Turn 关联及取消收束
├── persistence.rs  # 私有运行设置原子保存、Trace 关联字段校验与调用账本分段恢复
└── settings.rs  # 运行配置验证与搜索密钥目标绑定
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 7b75034578ff928841fb4d4a2bd4535eaefd12a846ccea27b55b53f50e1c7ca3 -->
