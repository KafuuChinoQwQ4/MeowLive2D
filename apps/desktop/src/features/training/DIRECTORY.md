# training 目录索引

音色训练素材审核、任务版本和离线测量界面

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
training/  # 音色训练素材审核、任务版本和离线测量界面
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── TrainingPanel.saved.test.tsx  # 音色保存、重开复用、跨音色版本切换和失败重试交互回归测试
├── TrainingPanel.test.tsx  # 训练提交审核、任务取消、试听确认和音频释放交互测试
├── TrainingPanel.tsx  # 训练素材审核、任务取消、版本试听保存、音色选择切换及离线预设面板
├── index.ts  # 训练面板公开组件导出
├── useTraining.test.tsx  # 训练轮询独立更新、故障恢复与取消迟到结果测试
└── useTraining.ts  # 训练资源预设独立轮询、音色保存切换及取消生命周期
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 217038e842bf1a07ae240ba0b959f5d6a5922e7fcb7db9db31eb9582e93d4e99 -->
