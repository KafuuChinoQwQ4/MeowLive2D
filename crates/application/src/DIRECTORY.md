# src 目录索引

Agent、事件调度、语音与资源任务用例

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
src/  # Agent、事件调度、语音与资源任务用例
├── agent/  # Agent 配置、输出校验、播放关联及状态类型
├── ports/  # 业务方定义的模型、语音、存储和执行能力边界
└── scheduler/  # 候选事件优先级与礼物分组策略
```

可继续查看各子目录的索引：

- [agent/](agent/DIRECTORY.md)：Agent 配置、输出校验、播放关联及状态类型
- [ports/](ports/DIRECTORY.md)：业务方定义的模型、语音、存储和执行能力边界
- [scheduler/](scheduler/DIRECTORY.md)：候选事件优先级与礼物分组策略

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 2dc0e4bb1bcc0fae6bcc8cafc2fec306560303b21ebf99bc8558c5528d6f81e6 -->
