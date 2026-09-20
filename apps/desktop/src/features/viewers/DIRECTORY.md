# viewers 目录索引

管理员只读观众档案与持久事件查询页面

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
viewers/  # 管理员只读观众档案与持久事件查询页面
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── MemoryCard.tsx  # 记忆证据、版本及纠正删除冻结操作卡片
├── RelationshipPanel.test.tsx  # 关系管理与图降级显示及请求幂等测试
├── RelationshipPanel.tsx  # 关系图、来源证据和确认撤销重建管理面板
├── ViewerDetail.test.tsx  # 观众管理详情、幂等重试和切换状态回归测试
├── ViewerDetail.tsx  # 管理员陪伴账本、礼物和记忆详情面板
├── ViewerMergePanel.test.tsx  # 身份合并显式确认与陈旧预览隔离测试
├── ViewerMergePanel.tsx  # 身份合并预览、风险确认和目标切换面板
├── ViewerPanel.test.tsx  # 观众与事件页面加载、分页及错误反馈测试
├── ViewerPanel.tsx  # 可展开收起并在框内滚动的观众与事件面板，分页展示身份、昵称历史、持久事件与未确认接收缺口
└── index.ts  # 观众查询功能组件的公共导出
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 66a7a1f55bebf9ba1ba65815aaa0c27b6a82a1e824b57ddd025a06b26d6cf0e2 -->
