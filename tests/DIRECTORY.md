# tests 目录索引

业务测试归属说明与回放数据入口

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
tests/  # 业务测试归属说明与回放数据入口
├── contracts/  # 跨端消息兼容及生成类型一致性测试的归属
├── e2e/  # Windows 实际播放和直播链路验证的归属
├── fixtures/  # 可提交的小型模拟和回放数据
└── integration/  # 用例与外部适配器集成测试的归属
```

可继续查看各子目录的索引：

- [contracts/](contracts/DIRECTORY.md)：跨端消息兼容及生成类型一致性测试的归属
- [e2e/](e2e/DIRECTORY.md)：Windows 实际播放和直播链路验证的归属
- [fixtures/](fixtures/DIRECTORY.md)：可提交的小型模拟和回放数据
- [integration/](integration/DIRECTORY.md)：用例与外部适配器集成测试的归属

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: ede1a038514b5714f571036ff427d187319b2aaca239804d01811d1566e6b50f -->
