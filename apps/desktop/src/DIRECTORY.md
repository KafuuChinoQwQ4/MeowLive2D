# src 目录索引

按应用组装、业务功能、外部服务和公共能力组织的前端源码

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
src/  # 按应用组装、业务功能、外部服务和公共能力组织的前端源码
├── app/  # React 根页面组装与全局样式
├── features/  # 面向用户的功能模块，各自封装组件与状态
├── services/  # 前端访问主服务、Linux 启动管理与 Windows 桌面能力的统一边界
├── shared/  # 跨功能复用且不持有业务流程的前端能力
└── test/  # 前端测试公共设施
```

可继续查看各子目录的索引：

- [app/](app/DIRECTORY.md)：React 根页面组装与全局样式
- [features/](features/DIRECTORY.md)：面向用户的功能模块，各自封装组件与状态
- [services/](services/DIRECTORY.md)：前端访问主服务、Linux 启动管理与 Windows 桌面能力的统一边界
- [shared/](shared/DIRECTORY.md)：跨功能复用且不持有业务流程的前端能力
- [test/](test/DIRECTORY.md)：前端测试公共设施

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 2acd97a6179dc29570d2f7b5538402b0364c66688754ef783146b63180ec5c92 -->
