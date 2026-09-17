# tests 目录索引

领域对象与输入不变量集成测试

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
tests/  # 领域对象与输入不变量集成测试
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── event_validation.rs  # 直播事件输入边界测试
├── resource_validation.rs  # 资源名称、语言、素材元数据与能力映射校验测试
└── speech_validation.rs  # 播报文本长度、空白、控制字符与音色标识校验测试
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 43e6d401d1b4110c0244a5d02783809cdb7dc409f1afd693ae92706637519877 -->
