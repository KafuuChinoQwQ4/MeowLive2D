# tests 目录索引

通信协议独立集成测试

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
tests/  # 通信协议独立集成测试
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── agent_contracts.rs  # Agent 输入与观察契约的严格反序列化及公开 JSON 形状测试
├── audio_frames.rs  # PCM 二进制帧编码、边界与损坏输入测试
├── compatibility.rs  # 协议必填字段、训练模式缺省兼容、未知标签与音频格式测试
├── control_serialization.rs  # 控制消息与执行回执序列化测试
└── obs_contracts.rs  # OBS 指令及设置请求未知字段拒绝、凭据脱敏与桌面资源契约测试
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 82faa3f12feab8972711a73736ab49a313dd6e97fa3f8a87885f1f15f4570628 -->
