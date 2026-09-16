# connection 目录索引

使用虚拟时间和真实 WebSocket 的连接期限测试

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
connection/  # 使用虚拟时间和真实 WebSocket 的连接期限测试
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── flush.rs  # 阻塞 Pong 写回的超时测试
└── liveness.rs  # 双通道独立心跳期限、续期及设备清理测试
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 14c98f8d61600d485e4f59445c8de24e57418c2d3dff9c00cfd00f06c3a6fb3f -->
