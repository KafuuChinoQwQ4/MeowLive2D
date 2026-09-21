# agent 目录索引

Agent 人设、话题和互动策略设置

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
agent/  # Agent 人设、话题和互动策略设置
├── AgentPanel.test.tsx  # Agent 状态控制、错误呈现、轮询竞态与取消清理测试
├── AgentPanel.tsx  # Agent 运行条件、暂停恢复、设置、事件输入与历史的组合面板
├── AgentSettingsForm.bounds.test.tsx  # 默认空话题及人设话题长度与后端一致性的回归测试
├── AgentSettingsForm.interaction.test.tsx  # 互动策略表单持久化、草稿保留与阈值边界测试
├── AgentSettingsForm.test.tsx  # Agent 设置草稿、输入校验与毫秒请求映射测试
├── AgentSettingsForm.tsx  # 保留草稿并编辑人设、冷却和弹幕欢迎互动策略的 Agent 设置表单
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── EventHistory.test.tsx  # Agent 事件内容、状态、播报关联与错误展示测试
├── EventHistory.tsx  # Agent 事件状态、关联播报及错误历史列表
├── EventSimulator.test.tsx  # 模拟事件校验、成功清空、失败保留与重复反馈测试
├── EventSimulator.tsx  # 聊天、礼物、SC 与进房模拟及 JSON 批量回放表单
├── index.ts  # Agent 自动互动面板的功能出口
└── useAgentController.ts  # 修订号防回滚及卸载取消的串行 Agent 轮询与动作控制器
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: fb22f456df4309cc4795d217e688e36444682c2be9bca1ce39005323a821494f -->
