# memory 目录索引

记忆记录、后台租约和向量分层持久实现

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
memory/  # 记忆记录、后台租约和向量分层持久实现
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── operations.rs  # 审计化任务恢复与向量重建事务
├── queue.rs  # 记忆提取任务租约、重试及来源复核
├── records.rs  # 记忆证据合并、时效检索和管理失效事务
└── vectors.rs  # 模型维度正文版本隔离的 pgvector 存取
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: f8b4e5586841c337eda30ddaa7ad887919641c8e9013bb7013c2bd4c80ffa9b1 -->
