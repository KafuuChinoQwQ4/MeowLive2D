# llm-runtime 目录索引

模型运行能力、用量费用与 Agent 实时活动界面

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
llm-runtime/  # 模型运行能力、用量费用与 Agent 实时活动界面
├── AgentActivityPanel.test.tsx  # 活动更新和隐藏卸载取消轮询测试
├── AgentActivityPanel.tsx  # Agent 阶段、工具进度与网页来源展示
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── LlmRuntimePanel.test.tsx  # 运行设置、密钥绑定、费用和取消交互测试
├── LlmRuntimePanel.tsx  # 运行配置与用量折叠区入口
├── RuntimeIntegration.test.tsx  # LLM 与 Agent 原页面运行能力接入测试
├── RuntimeSettingsPanel.tsx  # 运行开关、搜索密钥及模型单价编辑与保存
├── UsagePanel.tsx  # 按日期和模型查询已报告用量与估算费用
├── llm-runtime.css  # 运行能力与用量界面的粉色响应式样式
├── runtime-fixtures.ts  # 运行配置、用量及活动测试样例
└── useRuntimeVisibility.ts  # 按工作区和文档可见性暂停运行请求
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 8c6d2d4e69342dafa91fa1cc13791d291c0e03f9fcb1680238d9c017b449c0de -->
