# icons 目录索引

沿用控制台绿色 M 标志的图标源及窗口构建资源

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
icons/  # 沿用控制台绿色 M 标志的图标源及窗口构建资源
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── icon.ico  # Windows 窗口与应用资源使用的 ICO 图标
├── icon.png  # Tauri 使用的 RGBA 窗口图标
└── icon.svg  # 绿色 M 应用图标的可编辑矢量源
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 380feeb389c1dc33fcf6c043864ed9ca386112f67fd30a9814142ef2aa5dcd54 -->
