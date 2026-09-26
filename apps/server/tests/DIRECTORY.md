# tests 目录索引

主服务配置、HTTP、桥接与完整播报集成测试

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
tests/  # 主服务配置、HTTP、桥接与完整播报集成测试
├── agent_support/  # Agent 服务集成测试公共夹具
├── live_support/  # 可控直播源与事件的服务生命周期测试支持
├── process_support/  # 真实主服务进程测试的隔离目录和音频构造辅助
└── support/  # 主服务集成测试公共夹具
```

可继续查看各子目录的索引：

- [agent_support/](agent_support/DIRECTORY.md)：Agent 服务集成测试公共夹具
- [live_support/](live_support/DIRECTORY.md)：可控直播源与事件的服务生命周期测试支持
- [process_support/](process_support/DIRECTORY.md)：真实主服务进程测试的隔离目录和音频构造辅助
- [support/](support/DIRECTORY.md)：主服务集成测试公共夹具

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 0a694530532813ed4352d8113bd42b8711d8d11a77f7c3de7c76ba27f046db7b -->
