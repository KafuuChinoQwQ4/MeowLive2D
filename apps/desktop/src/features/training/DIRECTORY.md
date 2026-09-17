# training 目录索引

仅音频与可选文本训练、转写校对、任务版本和离线测量界面

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
training/  # 仅音频与可选文本训练、转写校对、任务版本和离线测量界面
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── TrainingPanel.audio.test.tsx  # 训练片段多格式转换上传、异步重选、容量与导入失败回归测试
├── TrainingPanel.feedback.test.tsx  # 验证 GPU 失败提示、训练与测量结果弹窗、终态竞态去重及过期请求隔离
├── TrainingPanel.saved.test.tsx  # 音色保存重开切换、删除确认及清理失败重试交互测试
├── TrainingPanel.test.tsx  # 训练配置独立选择、服务忙碌时编辑、提交审核与试听取消交互测试
├── TrainingPanel.transcription.test.tsx  # 训练声音与文本模式、空白转写汇总弹窗、审核及过期请求回归测试
├── TrainingPanel.tsx  # 训练性能设置、记录分页、音色版本、模型开关及结果弹窗工作区
├── TrainingPanel.workspace.test.tsx  # 训练页签与素材分页、音色归组、性能参数及独立模型启停测试
├── TrainingResultDialog.tsx  # 训练成功失败结果弹窗、建议提示及键盘焦点恢复
├── TrainingVoiceLibrary.tsx  # 按参考音色归组的训练音色库、版本选择与单版本操作
├── index.ts  # 训练面板公开组件导出
├── trainingFeedback.ts  # 训练操作结果、任务终态通知及失败原因对应的改正建议
├── useTraining.test.tsx  # 训练轮询独立更新、故障恢复与取消迟到结果测试
└── useTraining.ts  # 训练资源与模型状态轮询、操作反馈、终态通知及异步取消保护
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 15c85f1efbf8e8176644b3a24712c656881a35faf2cde3e9b7f70ef44e2c116d -->
