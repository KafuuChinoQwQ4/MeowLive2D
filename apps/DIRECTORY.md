# apps 目录索引

可执行应用入口与依赖组装

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
apps/  # 可执行应用入口与依赖组装
├── desktop/  # React 控制面板及 Windows 桌面外壳
└── server/  # Linux / WSL 主服务入口、配置和传输边界
```

可继续查看各子目录的索引：

- [desktop/](desktop/DIRECTORY.md)：React 控制面板及 Windows 桌面外壳
- [server/](server/DIRECTORY.md)：Linux / WSL 主服务入口、配置和传输边界

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: a472b1264bd78cb04ebbcbcc6c1ceeeec1a5c232b081fc9880f7cf127d4ee8c6 -->
