# desktop 目录索引

React 控制面板及 Windows 桌面外壳

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
desktop/  # React 控制面板及 Windows 桌面外壳
├── src/  # 按应用组装、业务功能、外部服务和公共能力组织的前端源码
└── src-tauri/  # Windows Tauri 薄外壳、窗口配置与执行库生命周期
```

可继续查看各子目录的索引：

- [src/](src/DIRECTORY.md)：按应用组装、业务功能、外部服务和公共能力组织的前端源码
- [src-tauri/](src-tauri/DIRECTORY.md)：Windows Tauri 薄外壳、窗口配置与执行库生命周期

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 0324ddf659941583ad041f84f2ce56bccaf8ecc27a28973deb66e19230771259 -->
