# resources 目录索引

资源快照转换与参考音频校验实现

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
resources/  # 资源快照转换与参考音频校验实现
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── snapshot.rs  # 持久化资源快照的版本化 DTO 与领域转换
└── wav.rs  # 参考音频 PCM 格式、时长和静音校验
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 27c47949795a98d1f854415eedea309ec16b012b7c820330502ccbd2d640f616 -->
