# adapters 目录索引

外部服务和存储实现，适配 application 定义的能力

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
adapters/  # 外部服务和存储实现，适配 application 定义的能力
├── migrations/  # PostgreSQL 观众身份与直播事件模式迁移
│   ├── 0001_viewer_identity_events.sql  # 观众、稳定身份、昵称、直播场次和幂等原始事件的 PostgreSQL 初始模式
│   ├── 0002_companionship.sql  # 来访礼物陪伴账本及完成回执数据迁移
│   ├── 0003_memories.sql  # 记忆事实证据向量及持久提取任务迁移
│   ├── 0004_relationships.sql  # 关系事实审计与图事务发件箱迁移
│   ├── 0005_memory_cleanup.sql  # 观众记忆外键级联清理规则增量迁移
│   ├── 0006_memory_operations.sql  # 记忆任务恢复和向量重建审计迁移
│   ├── 0007_viewer_merge.sql  # 身份合并墓碑和原始账本归属审计迁移
│   ├── 0008_memory_graph_invalidation.sql  # 记忆失效同事务生成关系墓碑与图同步任务的触发器
│   ├── 0009_superchat_room_enter_events.sql  # 扩展原始直播事件类型约束以支持醒目留言和进房事件
│   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── src/  # 按外部能力组织的适配器源码
│   ├── live/  # 直播源连接、事件标准化与模拟输入
│   │   ├── bilibili/  # 哔哩哔哩官方直播开放平台的授权、签名、连接与事件适配
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── config.rs  # 哔哩哔哩接入凭据、网络地址与资源上限配置校验
│   │   │   ├── connection.rs  # 官方直播长连接鉴权、接收、心跳与显式会话清理
│   │   │   ├── http.rs  # 开放平台签名 HTTP 项目开始、心跳、结束与错误处理
│   │   │   ├── mod.rs  # 哔哩哔哩直播适配器的模块与配置公开出口
│   │   │   ├── protocol.rs  # 官方直播二进制包解析、有界解压、鉴权回复与领域事件转换
│   │   │   └── signing.rs  # 开放平台请求体 MD5 与规范请求头 HMAC-SHA256 签名
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── mod.rs  # 直播源接入和事件标准化；去重、话题筛选与礼物合并属于 application。
│   │   └── simulator.rs  # 模拟弹幕、礼物和连接变化的事件来源，用于首个互动闭环与事件回放。
│   ├── llm/  # 云端及本地 LLM 的协议适配
│   │   ├── models/  # 模型目录配置及响应解析实现
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── config.rs  # 模型目录地址规范化与密钥校验
│   │   │   └── response.rs  # 供应商模型条目及分页元数据解析
│   │   ├── reasoning/  # 具体模型推理能力登记与原生参数映射
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── capabilities.rs  # 经官方文档核对的模型推理档位和预算预设能力表
│   │   ├── runtime/  # 四协议工具调用、流式响应、缓存和用量的统一适配
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── mod.rs  # 有界单轮模型调用与只读流式观测
│   │   │   ├── output.rs  # 原生输出、工具签名与用量归一化
│   │   │   ├── request.rs  # 原生工具、续接上下文与稳定缓存前缀请求
│   │   │   └── stream.rs  # 四协议 SSE 拼包、结束验证与用量保留
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── config.rs  # LLM 地址规范化、认证头及资源限制的统一校验
│   │   ├── mod.rs  # 模型协议适配。兼容同一协议的云端与本地服务复用实现，其他协议独立添加。
│   │   ├── models.rs  # 受限的供应商模型目录 HTTP 适配与分页汇总
│   │   ├── multi_provider.rs  # 多提供商决策与原生工具、流式运行层适配
│   │   ├── openai_compatible.rs  # 非流式 Chat Completions 传输、认证、响应大小与临时错误分类
│   │   ├── prompt.rs  # 跨 LLM 协议共享的事件回复与主动发言提示隔离及输入校验
│   │   ├── reasoning.rs  # 推理档位夹取、预算限制和各协议原生请求参数注入
│   │   └── response.rs  # LLM 文本决策内容与 OpenAI 聊天响应的严格校验
│   ├── speech/  # GPT-SoVITS 等语音引擎的请求和音频格式适配
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── gpt_sovits.rs  # GPT-SoVITS HTTP 适配入口：参考素材路径解析、合成参数映射与音频解码。
│   │   ├── mod.rs  # 语音引擎适配与音频格式转换。推理服务和模型权重位于项目目录之外。
│   │   ├── model_synthesizer.rs  # 模型内存启停、成对权重加载与合成互斥及取消事务保护
│   │   ├── resource_synthesizer.rs  # 根据音色档案解析参考音频并调用 GPT-SoVITS
│   │   └── wav.rs  # 完整 RIFF/WAV 边界与 PCM16 格式校验解码
│   ├── storage/  # PostgreSQL 观众事件与 Linux 素材文件存储
│   │   ├── postgres/  # 观众持久业务、记忆与关系的 PostgreSQL 实现
│   │   │   ├── memory/  # 记忆记录、后台租约和向量分层持久实现
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── operations.rs  # 审计化任务恢复与向量重建事务
│   │   │   │   ├── queue.rs  # 记忆提取任务租约、重试及来源复核
│   │   │   │   ├── records.rs  # 记忆证据合并、时效检索和管理失效事务
│   │   │   │   └── vectors.rs  # 模型维度正文版本隔离的 pgvector 存取
│   │   │   ├── relationships/  # 关系图投影租约与恢复实现目录
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── derived.rs  # 从可靠记忆派生明确兴趣、活动及未解析提及的事务规则
│   │   │   │   └── queue.rs  # 关系图事务发件箱认领重试和重建
│   │   │   ├── viewer_merge/  # 身份合并内部数据迁移实现
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   └── migrate.rs  # 合并身份时的去重迁移与账本对账
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── companionship.rs  # 原子来访礼物计分及幂等完成回执账本
│   │   │   ├── memory.rs  # 记忆持久端口实现与事务通用约束
│   │   │   ├── relationships.rs  # 权威关系事实、来源确认和版本化管理事务
│   │   │   └── viewer_merge.rs  # 有预览指纹与审计保护的身份合并事务
│   │   ├── resources/  # 资源快照转换与参考音频校验实现
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── snapshot.rs  # 资源快照版本化转换及音色参考标识一致性验证
│   │   │   └── wav.rs  # 参考音频 PCM 格式、时长和静音校验
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── files.rs  # Linux 素材文件存储，负责文件落盘及引擎可访问路径；Windows 模型导入另属 desktop-runtime。
│   │   ├── mod.rs  # 持久化和本地素材存储实现；应用层只看存取接口。
│   │   ├── postgres.rs  # 观众身份事件幂等接收与查询、迁移连接及业务存储集成
│   │   ├── receipt_journal.rs  # 同步持久、容量有界及公平重试的本地完成回执日志
│   │   ├── resources.rs  # 原子资源快照、音色与参考标识一致性校验及参考 WAV 存取清理
│   │   └── sqlite.rs  # SQLite 记录存储适配入口。表结构和迁移随首个持久化用例加入。
│   ├── training/  # 独立训练进程、进度及产物的适配
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── mod.rs  # 独立训练进程适配，处理进程状态与训练产物；调度策略属于 application。
│   │   ├── process.rs  # 独立训练进程组启动、超时取消和有界日志进度
│   │   ├── store.rs  # 任务素材与续训权重私有副本、原子历史存储和成对模型校验
│   │   ├── tests.rs  # 训练音色持久化、音频模式、转写进程、删除清理及存储故障测试
│   │   └── transcription.rs  # 单片音频自动转写的有界子进程、私有临时素材与退出清理
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── graph.rs  # Neo4j Query API 参数化投影和有界邻居检索
│   ├── lib.rs  # 外部能力实现：依赖业务层定义的 ports，不反向定义业务规则。
│   ├── memory.rs  # 独立 HTTP 记忆提取和嵌入服务适配器
│   ├── runtime.rs  # GPU 采样、WSL 工具查找与有界错误诊断
│   └── search.rs  # Brave 与 SearXNG 有界检索、来源过滤和错误脱敏
├── tests/  # GPT-SoVITS、WAV 与 LLM 适配器的集成和输入输出边界测试
│   ├── bilibili_support/  # 受控官方直播 HTTP 和 WebSocket 协议测试服务
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   └── mod.rs  # 官方直播模拟响应、鉴权帧、事件和生命周期观测夹具
│   ├── llm_support/  # LLM 适配器测试公共夹具
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   └── mod.rs  # 受控 LLM HTTP 服务、配置、事件和完成响应夹具
│   ├── support/  # 语音适配测试的 WAV 和 HTTP 公共夹具
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── http.rs  # 受控 TTS HTTP 服务与请求配置夹具
│   │   ├── mod.rs  # 内存 WAV 生成与公共夹具模块导出
│   │   └── raw_http.rs  # 分块响应测试所用原始 HTTP 请求读取夹具
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── bilibili_connection.rs  # 官方直播源签名请求、长连接授权、事件与清理行为测试
│   ├── bilibili_messages.rs  # 直播消息包头、压缩边界、事件映射与无效输入测试
│   ├── bilibili_resilience.rs  # 直播心跳互不阻塞、鉴权同包事件、失败清理、事件标识和总解压预算回归测试
│   ├── bilibili_signing.rs  # 哔哩哔哩签名固定向量、请求体字节与输入验证测试
│   ├── gpt_sovits_http.rs  # GPT-SoVITS 参数映射、完整长 SC 音频响应和引擎失败测试
│   ├── gpt_sovits_limits.rs  # 引擎响应体上限、分块传输、总超时和重定向测试
│   ├── gpt_sovits_validation.rs  # 引擎配置、默认音色和播报文本输入校验测试
│   ├── llm_cancellation.rs  # 取消模型决策 future 后关闭在途 HTTP 连接测试
│   ├── llm_limits.rs  # 配置请求响应上限、总超时、错误分类与敏感信息脱敏测试
│   ├── llm_models.rs  # 模型目录认证、地址规范化和供应商分页集成测试
│   ├── llm_models_bounds.rs  # 模型目录资源上限、畸形响应及密钥保护边界测试
│   ├── llm_multi_provider.rs  # 多 LLM 协议的请求认证、输出校验和边界测试
│   ├── llm_output_validation.rs  # 严格决策字段、事件子集、工具调用、截断及内容约束测试
│   ├── llm_reasoning.rs  # 推理档位上下边界、缺档、默认与未知模型的解析回归测试
│   ├── llm_reasoning_http.rs  # 推理参数在原生 HTTP、流式和工具续传中的适配与上限测试
│   ├── llm_runtime.rs  # 四协议运行层、工具续接、缓存及流式取消回归测试
│   ├── llm_transport.rs  # 路径认证、消息角色、主动发言历史隔离、礼物分组提示与重定向测试
│   ├── memory_http.rs  # 提取嵌入 HTTP 格式限额与来源校验测试
│   ├── model_runtime.rs  # 模型关闭拒绝合成、启停后恢复及不污染权重状态的 HTTP 测试
│   ├── model_synthesizer.rs  # 取消后的权重合成互斥与默认模型恢复链路测试
│   ├── neo4j_graph.rs  # 真实 Neo4j 版本墓碑范围隔离及恢复测试
│   ├── postgres_companionship.rs  # 真实 PostgreSQL 陪伴日预算并发幂等测试
│   ├── postgres_derived_relations.rs  # 真实 PostgreSQL 记忆派生关系与删除撤销不复活测试
│   ├── postgres_memories.rs  # 真实 PostgreSQL 记忆生命周期租约和管理抑制测试
│   ├── postgres_relationships.rs  # 真实 PostgreSQL 关系证据图发件箱与隔离测试
│   ├── postgres_viewer_merge.rs  # 真实 PostgreSQL 身份合并预览去重和陈旧请求测试
│   ├── postgres_viewers.rs  # PostgreSQL 观众身份、事件去重、并发、范围隔离和批次原子性容器测试
│   ├── receipt_journal.rs  # 回执日志重启幂等、文件权限、容量和损坏诊断测试
│   ├── resource_store.rs  # 参考音频存储持久化、删除重启、路径及标识一致性测试
│   ├── resource_synthesizer.rs  # 上传音色的引擎路径解析与默认音色回退测试
│   ├── wav_decoding.rs  # 完整 WAV 的基础 PCM 解码与无效输入测试
│   ├── wav_validation.rs  # WAV 采样率、位深、帧完整性和容器畸形校验测试
│   └── web_search.rs  # 搜索请求格式、来源过滤及响应长度边界测试
├── Cargo.toml  # 该 Rust 包的名称、workspace 配置与模块依赖声明
└── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
```

可继续查看各子目录的索引：

- [migrations/](migrations/DIRECTORY.md)：PostgreSQL 观众身份与直播事件模式迁移
- [src/](src/DIRECTORY.md)：按外部能力组织的适配器源码
- [tests/](tests/DIRECTORY.md)：GPT-SoVITS、WAV 与 LLM 适配器的集成和输入输出边界测试

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 52c3c2af2b282a2371cc8cc4d484904ef59781b23ed75d20fe88773336090e7c -->
