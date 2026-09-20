# storage 目录索引

PostgreSQL 观众事件与 Linux 素材文件存储

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
storage/  # PostgreSQL 观众事件与 Linux 素材文件存储
├── postgres/  # 观众持久业务、记忆与关系的 PostgreSQL 实现
│   ├── memory/  # 记忆记录、后台租约和向量分层持久实现
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── operations.rs  # 审计化任务恢复与向量重建事务
│   │   ├── queue.rs  # 记忆提取任务租约、重试及来源复核
│   │   ├── records.rs  # 记忆证据合并、时效检索和管理失效事务
│   │   └── vectors.rs  # 模型维度正文版本隔离的 pgvector 存取
│   ├── relationships/  # 关系图投影租约与恢复实现目录
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── derived.rs  # 从可靠记忆派生明确兴趣、活动及未解析提及的事务规则
│   │   └── queue.rs  # 关系图事务发件箱认领重试和重建
│   ├── viewer_merge/  # 身份合并内部数据迁移实现
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   └── migrate.rs  # 合并身份时的去重迁移与账本对账
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── companionship.rs  # 原子来访礼物计分及幂等完成回执账本
│   ├── memory.rs  # 记忆持久端口实现与事务通用约束
│   ├── relationships.rs  # 权威关系事实、来源确认和版本化管理事务
│   └── viewer_merge.rs  # 有预览指纹与审计保护的身份合并事务
├── resources/  # 资源快照转换与参考音频校验实现
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── snapshot.rs  # 资源快照版本化转换及音色参考标识一致性验证
│   └── wav.rs  # 参考音频 PCM 格式、时长和静音校验
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── files.rs  # Linux 素材文件存储，负责文件落盘及引擎可访问路径；Windows 模型导入另属 desktop-runtime。
├── mod.rs  # 持久化和本地素材存储实现；应用层只看存取接口。
├── postgres.rs  # 观众身份事件幂等接收与查询、迁移连接及业务存储集成
├── receipt_journal.rs  # 同步持久、容量有界及公平重试的本地完成回执日志
├── resources.rs  # 原子资源快照、音色与参考标识一致性校验及参考 WAV 存取清理
└── sqlite.rs  # SQLite 记录存储适配入口。表结构和迁移随首个持久化用例加入。
```

可继续查看各子目录的索引：

- [postgres/](postgres/DIRECTORY.md)：观众持久业务、记忆与关系的 PostgreSQL 实现
- [resources/](resources/DIRECTORY.md)：资源快照转换与参考音频校验实现

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: a31527bb399fb0f498ef5244b029a82bc13bc4fed47cb5fc84f4f9f2e256943c -->
