# assets 目录索引

Live2D 导出模型包的本地校验与安装

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
assets/  # Live2D 导出模型包的本地校验与安装
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
└── model.rs  # 模型清单与路径校验、无覆盖安装、模型身份枚举及持久化删除重试
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 2476a256a5b70584365d7f9b773f4dbb971eb3e0227b0a4b0fb4002d99631b2a -->
