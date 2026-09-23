# agent 目录索引

Agent 配置、输出校验、播放关联及状态类型

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
agent/  # Agent 配置、输出校验、播放关联及状态类型
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── decisions.rs  # 模型输出校验、结构化静默与作废结果、工作隔离和完整原文语音准备
├── interaction.rs  # 有界流量统计、弹幕朗读模式、进房欢迎冷却及原文播报前缀
├── playback.rs  # 播放状态同步、入队前取消和已完成对话记忆
├── settings.rs  # 人设配置和调度资源上限校验
└── types.rs  # Agent 阶段、事件状态、结构化等待原因及应用调用结果
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 5a36beb43a73ce819c3d9343057b8e71163d190bb77e55c210bfab66a0c276e8 -->
