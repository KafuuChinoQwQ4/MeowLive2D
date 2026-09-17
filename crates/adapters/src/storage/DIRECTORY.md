# storage 目录索引

SQLite 记录与 Linux 素材文件存储

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
storage/  # SQLite 记录与 Linux 素材文件存储
├── resources/  # 资源快照转换与参考音频校验实现
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── snapshot.rs  # 资源快照版本化转换及音色参考标识一致性验证
│   └── wav.rs  # 参考音频 PCM 格式、时长和静音校验
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── files.rs  # Linux 素材文件存储，负责文件落盘及引擎可访问路径；Windows 模型导入另属 desktop-runtime。
├── mod.rs  # 持久化和本地素材存储实现；应用层只看存取接口。
├── resources.rs  # 原子资源快照、音色与参考标识一致性校验及参考 WAV 存取清理
└── sqlite.rs  # SQLite 记录存储适配入口。表结构和迁移随首个持久化用例加入。
```

可继续查看各子目录的索引：

- [resources/](resources/DIRECTORY.md)：资源快照转换与参考音频校验实现

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 8278561903006f1f2668af8ef9cdf53e985262c7af1cab8f6491fa1462db349b -->
