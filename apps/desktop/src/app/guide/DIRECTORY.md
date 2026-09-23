# guide 目录索引

控制面板集中使用指南与各功能操作步骤

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
guide/  # 控制面板集中使用指南与各功能操作步骤
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── GuidePanel.tsx  # 独立新手指南、可展开的功能用法与工作区跳转入口
└── content.ts  # 启动、语音、角色、互动、Agent 观察、观众、直播和训练的中文使用步骤
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: dc584834933c6f2d291871594340d6881984f39f38e5198fb9899576399a592c -->
