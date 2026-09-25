# domain 目录索引

业务对象、状态和不变量，保持无外部依赖

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
domain/  # 业务对象、状态和不变量，保持无外部依赖
├── src/  # 事件、会话、角色、音色和执行状态的领域定义
└── tests/  # 领域对象与输入不变量集成测试
```

可继续查看各子目录的索引：

- [src/](src/DIRECTORY.md)：事件、会话、角色、音色和执行状态的领域定义
- [tests/](tests/DIRECTORY.md)：领域对象与输入不变量集成测试

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: eb0b370ccbdeebd6ffe2e20e3e317795b3db100ced64b212c1f1edf0a0db2e5b -->
