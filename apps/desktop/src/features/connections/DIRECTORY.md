# connections 目录索引

直播平台连接状态、事件计数和人工连接控制

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
connections/  # 直播平台连接状态、事件计数和人工连接控制
├── ConnectionPanel.test.tsx  # 直播连接面板状态展示、按钮规则与错误交互测试
├── ConnectionPanel.tsx  # 直播平台连接状态、事件统计与手动连接控制面板
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── index.ts  # 直播平台连接功能公共出口
├── polling.test.tsx  # 直播连接轮询串行、严格模式、迟到响应及卸载取消测试
└── useConnectionController.ts  # 直播连接轮询、操作互斥、取消及响应顺序控制器
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 67cc552fedd22d71aaede666cb16e7624bc6020023757508664e0cb535ae704f -->
