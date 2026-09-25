# storage 目录索引

PostgreSQL 观众事件与 Linux 素材文件存储

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
storage/  # PostgreSQL 观众事件与 Linux 素材文件存储
├── postgres/  # 观众持久业务、记忆与关系的 PostgreSQL 实现
└── resources/  # 资源快照转换与参考音频校验实现
```

可继续查看各子目录的索引：

- [postgres/](postgres/DIRECTORY.md)：观众持久业务、记忆与关系的 PostgreSQL 实现
- [resources/](resources/DIRECTORY.md)：资源快照转换与参考音频校验实现

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 493df32eba79fcc1076ec42a5f7151f0b15030d9ca69a58e754bef5ba20da8d0 -->
