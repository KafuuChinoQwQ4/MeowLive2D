# config 目录索引

按 Agent 与模型能力拆分的配置校验

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
config/  # 按 Agent 与模型能力拆分的配置校验
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── agent.rs  # Agent 人设和有界调度参数的 TOML 配置
├── auth.rs  # 管理员和设备私有凭据来源与会话期限配置校验
├── graph.rs  # 图副本连接和私有凭据来源配置
├── live.rs  # 直播接入开关、兼容环境变量与本机覆盖凭据的校验和脱敏
├── llm.rs  # LLM 提供商协议、地址、模型及资源限额校验与密钥脱敏
├── memory.rs  # 独立提取嵌入模型与请求预算配置
├── resources.rs  # Linux 资源保存目录和引擎共享挂载配置
├── training.rs  # 训练路径、本地识别模型、超时及受管推理默认权重配置校验
└── viewers.rs  # 默认开启的观众与记忆持久化、角色范围和数据库环境变量配置校验
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: dc542363d0930782dbc78185210c4123143cdf7b6b429a255949d7e0b8491530 -->
