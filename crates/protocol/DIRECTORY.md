# protocol 目录索引

跨进程控制、事件、音频和执行消息的契约源

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
protocol/  # 跨进程控制、事件、音频和执行消息的契约源
├── src/  # 与业务领域分离的通信 DTO 模块
└── tests/  # 通信协议独立集成测试
```

可继续查看各子目录的索引：

- [src/](src/DIRECTORY.md)：与业务领域分离的通信 DTO 模块
- [tests/](tests/DIRECTORY.md)：通信协议独立集成测试

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 11131d619349cca572a121455f6fedf28d41413ed93efb2294eedd009ab856ac -->
