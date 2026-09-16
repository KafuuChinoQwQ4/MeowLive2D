# live 目录索引

官方直播源组装及异步连接、接收、清理与重连驱动

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
live/  # 官方直播源组装及异步连接、接收、清理与重连驱动
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── bootstrap.rs  # 从主服务环境变量组装官方直播适配器
└── worker.rs  # 直播事件接收、去重入队、取消、会话清理和退避重连
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: c42ac01dec0e0fcc883f8f19288b41d9224fd0d462db82aa9a8c2cc01091daca -->
