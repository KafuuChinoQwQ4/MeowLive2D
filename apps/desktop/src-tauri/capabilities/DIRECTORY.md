# capabilities 目录索引

桌面窗口的 Tauri 能力声明

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
capabilities/  # 桌面窗口的 Tauri 能力声明
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
└── default.json  # 主窗口的最小权限范围，未开放系统插件能力
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: d3cf19a05558a9c810c1a92334f52343b1f640682f964e5cf99ef10205542eb0 -->
