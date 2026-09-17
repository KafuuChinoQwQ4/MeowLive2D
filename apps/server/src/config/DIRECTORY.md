# config 目录索引

按 Agent 与模型能力拆分的配置校验

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
config/  # 按 Agent 与模型能力拆分的配置校验
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── agent.rs  # Agent 人设和有界调度参数的 TOML 配置
├── live.rs  # 官方直播接入开关、应用编号、凭据变量名和重连参数校验
├── llm.rs  # LLM 提供商协议、地址、模型及资源限额校验与密钥脱敏
├── resources.rs  # Linux 资源保存目录和引擎共享挂载配置
└── training.rs  # 训练路径、本地识别模型、超时及受管推理默认权重配置校验
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 0aefac1d859678773fbd9230a5c7e8da339e2ebb5bd7ff2831f831bbe59c8889 -->
