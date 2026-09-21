# llm_runtime 目录索引

运行配置、计量与持久化实现

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
llm_runtime/  # 运行配置、计量与持久化实现
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── ledger.rs  # 历史用量筛选聚合与缓存子集费用估算
├── metering.rs  # 单次模型调用观测计量及取消收束
├── persistence.rs  # 私有运行设置原子保存与调用账本分段恢复
└── settings.rs  # 运行配置验证与搜索密钥目标绑定
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: d8b79d3b87ca1b5ab26c3bd8473e449aa706d11fa70c19c91b6a002586475809 -->
