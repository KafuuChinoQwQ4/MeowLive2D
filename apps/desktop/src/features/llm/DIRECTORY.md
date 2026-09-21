# llm 目录索引

LLM 接入配置功能

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
llm/  # LLM 接入配置功能
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── LlmPanel.models.test.tsx  # LLM 模型发现、自动识别、选择与连接切换交互测试
├── LlmPanel.reasoning.test.tsx  # 推理档位选择、映射展示、预算阻断与请求竞态回归测试
├── LlmPanel.test.tsx  # LLM 面板草稿、密钥与生命周期测试
├── LlmPanel.tsx  # LLM 连接配置、模型获取与选择、推理档位预览和测试保存面板
├── index.ts  # LLM 功能公共入口
└── useReasoningPreview.ts  # 推理能力预览的防抖、取消、重试与迟到响应隔离
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: f643ad1326a0e07c132ba0fde3be8c0140f224737066da42270970b654784d6a -->
