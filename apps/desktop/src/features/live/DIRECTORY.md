# live 目录索引

直播工作台、弹幕观察与播报控制

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
live/  # 直播工作台、弹幕观察与播报控制
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── SpeechPanel.scenarios.test.tsx  # 提交、停止、失败与重复操作场景测试
├── SpeechPanel.test.tsx  # 输入校验、可用状态和历史展示组件测试
├── SpeechPanel.tsx  # 中文人工语音播报控制面板
├── index.ts  # 直播工作台：会话状态、弹幕观察、播放状态与人工控制。
├── polling.test.tsx  # 状态刷新、取消清理与迟到响应场景测试
└── useSpeechController.ts  # 可取消的串行状态刷新与播报操作状态
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: e5b1800701c7c3518cb1d5f5234c42a21b3297007bb1caa5d8fe6f85a02ae394 -->
