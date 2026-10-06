# src 目录索引

桌面启动、依赖组装与命令转发源码

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
src/  # 桌面启动、依赖组装与命令转发源码
├── environment/  # Windows WSL 与 Linux 本机语音环境检测和配置保护
├── platform/  # 操作系统专属桌面状态、启动及 IPC
└── updates/  # GitHub 签名更新与分块增量下载及跨平台安装
```

可继续查看各子目录的索引：

- [environment/](environment/DIRECTORY.md)：Windows WSL 与 Linux 本机语音环境检测和配置保护
- [platform/](platform/DIRECTORY.md)：操作系统专属桌面状态、启动及 IPC
- [updates/](updates/DIRECTORY.md)：GitHub 签名更新与分块增量下载及跨平台安装

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: ff83fef65e1caa54b4a7dba80abbe794e63567002d472b162bf173edfddeeb48 -->
