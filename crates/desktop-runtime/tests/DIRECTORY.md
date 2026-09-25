# tests 目录索引

桌面执行运行时独立集成测试

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
tests/  # 桌面执行运行时独立集成测试
├── avatar_support/  # VTS 本地 WebSocket 场景测试的隔离文件与协议辅助设施
├── connection/  # 使用虚拟时间和真实 WebSocket 的连接期限测试
├── lip_sync_support/  # 口型观察测试的共享可控设备夹具
└── support/  # 运行时测试共用手动设备与消息夹具
```

可继续查看各子目录的索引：

- [avatar_support/](avatar_support/DIRECTORY.md)：VTS 本地 WebSocket 场景测试的隔离文件与协议辅助设施
- [connection/](connection/DIRECTORY.md)：使用虚拟时间和真实 WebSocket 的连接期限测试
- [lip_sync_support/](lip_sync_support/DIRECTORY.md)：口型观察测试的共享可控设备夹具
- [support/](support/DIRECTORY.md)：运行时测试共用手动设备与消息夹具

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 82f24164d7da9d52076ef08802b2d613b1175788ce5cf4bf334d8c633487beca -->
