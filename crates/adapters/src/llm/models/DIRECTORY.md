# models 目录索引

模型目录配置及响应解析实现

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
models/  # 模型目录配置及响应解析实现
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── config.rs  # 模型目录地址规范化与密钥校验
└── response.rs  # 供应商模型条目及分页元数据解析
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 8c67bb53ee2112607c0209d7b612650391a58eb88e60b50cddf35fd6137e725d -->
