# bin 目录索引

协议开发命令入口

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
bin/  # 协议开发命令入口
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
└── export-types.rs  # 从 Rust DTO 生成并检查含 Agent 观察模型的 TypeScript 契约
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 2dd213138529e663045b1b5ee4e5cee1d51c557631ac770a71ea832bbd038e5d -->
