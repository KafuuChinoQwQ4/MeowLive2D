# fixtures 目录索引

可提交的小型模拟和回放数据

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
fixtures/  # 可提交的小型模拟和回放数据
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── README.md  # 小型事件回放素材的内容边界及确定性要求
└── agent-events.json  # 聊天及连续礼物的可粘贴回放样本，含重复 ID 去重场景
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 9085403d1bdd58ba11ff84e513b459d21ff81a7fd8aa9ad77df1bfd25b1a4e34 -->
