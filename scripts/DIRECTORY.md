# scripts 目录索引

开发工具、目录用途登记与索引同步检查

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
scripts/  # 开发工具、目录用途登记与索引同步检查
└── launcher/  # Linux 本机服务启动管理、配置读取、状态探测与控制接口
```

可继续查看各子目录的索引：

- [launcher/](launcher/DIRECTORY.md)：Linux 本机服务启动管理、配置读取、状态探测与控制接口

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 6006abb7f2b942e1150f4d2eecf31f5900df09f67249b5803d4667a3edc6e059 -->
