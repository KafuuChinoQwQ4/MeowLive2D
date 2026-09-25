# crates 目录索引

按职责与单向依赖隔离的 Rust 库

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
crates/  # 按职责与单向依赖隔离的 Rust 库
├── adapters/  # 外部服务和存储实现，适配 application 定义的能力
├── application/  # 业务用例编排及外部能力接口定义
├── desktop-runtime/  # 独立于界面的 Windows 播放与设备执行库
├── domain/  # 业务对象、状态和不变量，保持无外部依赖
└── protocol/  # 跨进程控制、事件、音频和执行消息的契约源
```

可继续查看各子目录的索引：

- [adapters/](adapters/DIRECTORY.md)：外部服务和存储实现，适配 application 定义的能力
- [application/](application/DIRECTORY.md)：业务用例编排及外部能力接口定义
- [desktop-runtime/](desktop-runtime/DIRECTORY.md)：独立于界面的 Windows 播放与设备执行库
- [domain/](domain/DIRECTORY.md)：业务对象、状态和不变量，保持无外部依赖
- [protocol/](protocol/DIRECTORY.md)：跨进程控制、事件、音频和执行消息的契约源

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 2a212c9525159fcd177d9c64feae7bb8475d5c0cf638b18409824a2dd0f5c11d -->
