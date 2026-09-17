# resources 目录索引

资源快照转换与参考音频校验实现

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
resources/  # 资源快照转换与参考音频校验实现
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── snapshot.rs  # 资源快照版本化转换及音色参考标识一致性验证
└── wav.rs  # 参考音频 PCM 格式、时长和静音校验
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 6c351d239ec451c7e50976d69767bede3a67caf05f93ec902ae6614c2d1ade88 -->
