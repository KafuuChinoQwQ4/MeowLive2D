# obs 目录索引

OBS 本地连接配置与校验

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
obs/  # OBS 本地连接配置与校验
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── config.rs  # OBS 本机 WebSocket 连接、兼容环境变量和私有设置路径校验
└── settings.rs  # 执行端 OBS 设置私有文件读写、凭据保留清除及公开状态脱敏
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: b91a6a7a06c91430718a570105fa0f7456a68594de584c51dae04eb647701024 -->
