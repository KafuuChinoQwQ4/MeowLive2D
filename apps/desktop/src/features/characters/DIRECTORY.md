# characters 目录索引

角色模型选择、导入与动作映射界面

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
characters/  # 角色模型选择、导入与动作映射界面
├── CharacterPanel.test.tsx  # 角色与安装模型删除确认、导入保存加载及能力预览交互测试
├── CharacterPanel.tsx  # 角色配置及安装模型增删、音色绑定、口型和热键管理界面
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── index.ts  # 角色管理：VTS 模型选择、表情动作映射及导入操作的界面。
└── types.ts  # 角色档案与本机模型增删管理界面的状态和能力契约
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: e183d53e86f8cfca77dec26e439e8b1f3aaf235f14d6427e8eb8b1e8860ab4af -->
