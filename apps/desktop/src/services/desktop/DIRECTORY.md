# desktop 目录索引

Tauri 命令客户端与桌面能力边界

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
desktop/  # Tauri 命令客户端与桌面能力边界
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── index.test.ts  # 桌面 IPC 状态校验、来源约束和超时回归测试
└── index.ts  # 浏览器与 Tauri 环境识别及有界桌面状态读取
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 1847d01e15ee981f4d4b92b90ee05272f685a1bd5a6944bd7ec7254881982bdb -->
