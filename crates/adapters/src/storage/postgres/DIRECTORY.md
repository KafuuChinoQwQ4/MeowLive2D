# postgres 目录索引

观众持久业务、记忆与关系的 PostgreSQL 实现

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
postgres/  # 观众持久业务、记忆与关系的 PostgreSQL 实现
├── memory/  # 记忆记录、后台租约和向量分层持久实现
├── relationships/  # 关系图投影租约与恢复实现目录
└── viewer_merge/  # 身份合并内部数据迁移实现
```

可继续查看各子目录的索引：

- [memory/](memory/DIRECTORY.md)：记忆记录、后台租约和向量分层持久实现
- [relationships/](relationships/DIRECTORY.md)：关系图投影租约与恢复实现目录
- [viewer_merge/](viewer_merge/DIRECTORY.md)：身份合并内部数据迁移实现

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 41815dd06d0f8c5c4bc01c92466589774e122b7caaf02f03c93e4dad084a9797 -->
