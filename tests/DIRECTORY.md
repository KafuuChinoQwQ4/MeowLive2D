# tests 目录索引

业务测试归属说明与回放数据入口

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
tests/  # 业务测试归属说明与回放数据入口
├── contracts/  # 跨端消息兼容及生成类型一致性测试的归属
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── README.md  # 协议序列化、版本兼容与类型一致性的测试规划
├── e2e/  # Windows 实际播放和直播链路验证的归属
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── README.md  # Windows 播放口型、SC 与流量互动、停止及重连的端到端验证约定
├── fixtures/  # 可提交的小型模拟和回放数据
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── README.md  # 小型事件回放素材的内容边界及确定性要求
│   └── agent-events.json  # 聊天及连续礼物的可粘贴回放样本，含重复 ID 去重场景
├── integration/  # 用例与外部适配器集成测试的归属
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── README.md  # 模型适配与业务用例的模拟和真实服务验证约定
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
└── README.md  # 业务测试分层、执行入口和工具测试位置说明
```

可继续查看各子目录的索引：

- [contracts/](contracts/DIRECTORY.md)：跨端消息兼容及生成类型一致性测试的归属
- [e2e/](e2e/DIRECTORY.md)：Windows 实际播放和直播链路验证的归属
- [fixtures/](fixtures/DIRECTORY.md)：可提交的小型模拟和回放数据
- [integration/](integration/DIRECTORY.md)：用例与外部适配器集成测试的归属

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 44f24f0477a06bff9c0935ced3be1b36b0b128c72330b80df864f600790293a1 -->
