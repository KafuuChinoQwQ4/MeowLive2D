# src-tauri 目录索引

Windows Tauri 薄外壳、窗口配置与执行库生命周期

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
src-tauri/  # Windows Tauri 薄外壳、窗口配置与执行库生命周期
├── capabilities/  # 桌面窗口的 Tauri 能力声明
├── icons/  # 沿用控制台绿色 M 标志的图标源及窗口构建资源
├── src/  # 桌面启动、依赖组装与命令转发源码
└── tests/  # 可跨平台验证的桌面配置与启动测试
```

可继续查看各子目录的索引：

- [capabilities/](capabilities/DIRECTORY.md)：桌面窗口的 Tauri 能力声明
- [icons/](icons/DIRECTORY.md)：沿用控制台绿色 M 标志的图标源及窗口构建资源
- [src/](src/DIRECTORY.md)：桌面启动、依赖组装与命令转发源码
- [tests/](tests/DIRECTORY.md)：可跨平台验证的桌面配置与启动测试

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 873881014ab4a51e3d11c34cd59d7fbf39a7c788131dacde4520fa7281d20b09 -->
