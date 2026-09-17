# obs 目录索引

OBS 场景与录制控制面板

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
obs/  # OBS 场景与录制控制面板
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── ObsPanel.test.tsx  # OBS 显式控制、状态读回和失败后禁用操作的组件测试
├── ObsPanel.tsx  # OBS 状态刷新、场景选择和人工录制控制界面
└── index.ts  # OBS 功能模块的公开组件出口
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 7e8dd4c4d24316796913b9111c5cc0749da072eaf17e7fb0c40df11936b9ff5a -->
