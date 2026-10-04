# src 目录索引

桌面启动、依赖组装与命令转发源码

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
src/  # 桌面启动、依赖组装与命令转发源码
├── environment/  # Windows WSL 语音环境探测、安装、模型及进程管理
└── updates/  # GitHub 签名更新与分块增量下载
```

可继续查看各子目录的索引：

- [environment/](environment/DIRECTORY.md)：Windows WSL 语音环境探测、安装、模型及进程管理
- [updates/](updates/DIRECTORY.md)：GitHub 签名更新与分块增量下载

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 3117716e1937453762136c979957732ea96d166efe5b3b0af674d960e27c99a8 -->
