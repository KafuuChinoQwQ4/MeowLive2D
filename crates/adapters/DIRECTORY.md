# adapters 目录索引

外部服务和存储实现，适配 application 定义的能力

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
adapters/  # 外部服务和存储实现，适配 application 定义的能力
├── migrations/  # PostgreSQL 观众身份与直播事件模式迁移
├── src/  # 按外部能力组织的适配器源码
└── tests/  # GPT-SoVITS、WAV 与 LLM 适配器的集成和输入输出边界测试
```

可继续查看各子目录的索引：

- [migrations/](migrations/DIRECTORY.md)：PostgreSQL 观众身份与直播事件模式迁移
- [src/](src/DIRECTORY.md)：按外部能力组织的适配器源码
- [tests/](tests/DIRECTORY.md)：GPT-SoVITS、WAV 与 LLM 适配器的集成和输入输出边界测试

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: d261577bffbdc5140d88d55894cefb81334460c61a2b0abf33dbca1a87254516 -->
