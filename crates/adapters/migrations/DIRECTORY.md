# migrations 目录索引

PostgreSQL 观众身份与直播事件模式迁移

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
migrations/  # PostgreSQL 观众身份与直播事件模式迁移
├── 0001_viewer_identity_events.sql  # 观众、稳定身份、昵称、直播场次和幂等原始事件的 PostgreSQL 初始模式
├── 0002_companionship.sql  # 来访礼物陪伴账本及完成回执数据迁移
├── 0003_memories.sql  # 记忆事实证据向量及持久提取任务迁移
├── 0004_relationships.sql  # 关系事实审计与图事务发件箱迁移
├── 0005_memory_cleanup.sql  # 观众记忆外键级联清理规则增量迁移
├── 0006_memory_operations.sql  # 记忆任务恢复和向量重建审计迁移
├── 0007_viewer_merge.sql  # 身份合并墓碑和原始账本归属审计迁移
├── 0008_memory_graph_invalidation.sql  # 记忆失效同事务生成关系墓碑与图同步任务的触发器
└── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: a9128226ec832a9e88a0b2a4e114ed940c2ab51f99528014167eca8a0b049ead -->
