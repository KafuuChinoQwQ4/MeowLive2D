# app 目录索引

React 根页面组装与全局样式

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
app/  # React 根页面组装与全局样式
├── feedback/  # 全局操作结果弹窗、去重与面板反馈回归测试
├── guide/  # 控制面板集中使用指南与各功能操作步骤
└── resources/  # 角色人物卡与音色页面组装、共享资源控制器
```

可继续查看各子目录的索引：

- [feedback/](feedback/DIRECTORY.md)：全局操作结果弹窗、去重与面板反馈回归测试
- [guide/](guide/DIRECTORY.md)：控制面板集中使用指南与各功能操作步骤
- [resources/](resources/DIRECTORY.md)：角色人物卡与音色页面组装、共享资源控制器

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: f350f11c199b08400353c2a0de7297ce7aec46ba1ba11780696351319add7ba5 -->
