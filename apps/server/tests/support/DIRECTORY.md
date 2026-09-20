# support 目录索引

主服务集成测试公共夹具

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
support/  # 主服务集成测试公共夹具
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
└── mod.rs  # 受控合成器、HTTP 请求与双 WebSocket 连接夹具
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 17e68a4594e88be0e3d44f6c055258f96b6b9e5d1c71f167973a014a6495bfe6 -->
