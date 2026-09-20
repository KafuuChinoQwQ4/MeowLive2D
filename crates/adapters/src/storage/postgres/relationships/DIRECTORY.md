# relationships 目录索引

关系图投影租约与恢复实现目录

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
relationships/  # 关系图投影租约与恢复实现目录
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── derived.rs  # 从可靠记忆派生明确兴趣、活动及未解析提及的事务规则
└── queue.rs  # 关系图事务发件箱认领重试和重建
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 2e34ed19025a0eebbb1056ce9f7e2d46c6dbf50bf63dc86a1eb87f69b39a563d -->
