# characters 目录索引

角色模型选择、导入与动作映射界面

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
characters/  # 角色模型选择、导入与动作映射界面
├── CharacterPanel.test.tsx  # 角色导入、保存、加载与能力预览交互测试
├── CharacterPanel.tsx  # 角色模型、音色、口型与热键映射管理界面
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── index.ts  # 角色管理：VTS 模型选择、表情动作映射及导入操作的界面。
└── types.ts  # 角色面板需要的状态与操作接口
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: cb4d0345646777b200f3619035daa1ebca39a1b484664454dd5a288f89409e5d -->
