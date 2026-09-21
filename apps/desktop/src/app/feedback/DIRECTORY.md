# feedback 目录索引

全局操作结果弹窗、去重与面板反馈回归测试

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
feedback/  # 全局操作结果弹窗、去重与面板反馈回归测试
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── OperationFeedback.test.tsx  # 行内结果隔离、错误队列焦点恢复、后台故障去重及启动状态回归测试
├── OperationFeedback.tsx  # 分页面行内反馈、错误去重队列、无障碍错误弹窗与焦点恢复
├── PanelFeedback.test.tsx  # 各控制面板成功行内反馈、业务失败、异步状态与输入校验弹窗测试
└── feedback.css  # 页面行内结果文本及错误弹窗的配色、遮罩与响应式样式
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 0463a351d0fe1b9172ebd21addf83e0aa2996312a5468608d50ad037ce1ec67d -->
