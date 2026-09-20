# llm 目录索引

云端及本地 LLM 的协议适配

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
llm/  # 云端及本地 LLM 的协议适配
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── config.rs  # LLM 地址规范化、认证头及资源限制的统一校验
├── mod.rs  # 模型协议适配。兼容同一协议的云端与本地服务复用实现，其他协议独立添加。
├── multi_provider.rs  # OpenAI Responses、Anthropic Messages、Gemini 与兼容聊天的协议适配
├── openai_compatible.rs  # 非流式 Chat Completions 传输、认证、响应大小与临时错误分类
├── prompt.rs  # 跨 LLM 协议共享的事件回复与主动发言提示隔离及输入校验
└── response.rs  # LLM 文本决策内容与 OpenAI 聊天响应的严格校验
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: ba6c6921cd1b1583168ef766ba7d461242c8fcac9e4633d2a76a5bba76fdb3d0 -->
