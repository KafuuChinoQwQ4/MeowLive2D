# resources 目录索引

角色与音色功能的页面组装和共享状态

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
resources/  # 角色与音色功能的页面组装和共享状态
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── ResourcesPanel.lifecycle.test.tsx  # 资源读取与操作的卸载取消测试
├── ResourcesPanel.preview.test.tsx  # 音色试听状态跟踪、执行端断线、失败与重试的前端回归测试
├── ResourcesPanel.regressions.test.tsx  # 当前角色重新加载与安装成功后刷新失败回归
├── ResourcesPanel.tsx  # 组装角色音色面板与同服务本地转写客户端并显示资源操作状态
├── index.ts  # 资源管理页面公共入口
└── useResourcesController.ts  # 角色音色快照、资源删除及桌面模型管理共享控制器
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 70b81c92828bba2e13b1878373c54eff2685caae1595076b9a65efb6ef5f1d04 -->
