# resources 目录索引

角色人物卡与音色页面组装、共享资源控制器

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
resources/  # 角色人物卡与音色页面组装、共享资源控制器
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── ResourcesPanel.lifecycle.test.tsx  # 资源读取与操作取消及训练选用后的音色刷新测试
├── ResourcesPanel.preview.test.tsx  # 音色试听状态跟踪、执行端断线、失败与重试的前端回归测试
├── ResourcesPanel.regressions.test.tsx  # 当前角色重新加载与安装成功后刷新失败回归
├── ResourcesPanel.tsx  # 按角色或音色模式组装人物卡、资源面板及切页和训练操作后的刷新
├── index.ts  # 资源管理页面公共入口
└── useResourcesController.ts  # 角色音色快照、切页刷新竞态、资源删除及桌面模型管理控制器
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: e2329273de8d3428ea495eafe2c8035823d7ca74b479a6346b2f3d3ce4d88700 -->
