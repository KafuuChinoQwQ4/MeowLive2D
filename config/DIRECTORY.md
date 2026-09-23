# config 目录索引

独立数据库部署配置与 Linux、Windows 配置示例

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
config/  # 独立数据库部署配置与 Linux、Windows 配置示例
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── README.md  # 数据库、主服务、长语音、直播互动策略和 Windows 执行端配置说明
├── agent-runtime.md  # Agent 原生协议、推理强度、缓存、搜索、Trace Turn 观察及用量估算说明
├── databases.compose.yaml  # MeowLive2D 独立 PostgreSQL、pgvector 与 Neo4j 的容器、端口、数据挂载和资源上限
├── databases.env.example  # 独立数据库的私有凭据变量格式示例，不含实际密码
├── desktop.example.toml  # Windows 执行端连接、缓冲、VTS 插件与口型的默认关闭示例配置
├── launcher.example.json  # 网页启动器的主服务配置、TTS 环境与本地密钥文件路径模板
├── offline.example.toml  # 本地兼容量化 LLM 与 TTS 的受限离线运行配置样例
├── postgres-init.sql  # 首次初始化观众数据库的 vector 扩展、受限应用账号和独立 schema
├── server.example.toml  # 主服务、长语音、默认暂停 Agent 互动策略、LLM 和直播接入示例参数
└── viewer-recovery.md  # 观众数据库备份、删除资料同步及副本重建流程
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 54b8d58d2628495ea1b105c5ca4c67af2dd6102bc2369d6be662fecf08055927 -->
