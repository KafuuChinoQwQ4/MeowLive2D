# crates 目录索引

按职责与单向依赖隔离的 Rust 库

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
crates/  # 按职责与单向依赖隔离的 Rust 库
├── adapters/  # 外部服务和存储实现，适配 application 定义的能力
│   ├── migrations/  # PostgreSQL 观众身份与直播事件模式迁移
│   │   ├── 0001_viewer_identity_events.sql  # 观众、稳定身份、昵称、直播场次和幂等原始事件的 PostgreSQL 初始模式
│   │   ├── 0002_companionship.sql  # 来访礼物陪伴账本及完成回执数据迁移
│   │   ├── 0003_memories.sql  # 记忆事实证据向量及持久提取任务迁移
│   │   ├── 0004_relationships.sql  # 关系事实审计与图事务发件箱迁移
│   │   ├── 0005_memory_cleanup.sql  # 观众记忆外键级联清理规则增量迁移
│   │   ├── 0006_memory_operations.sql  # 记忆任务恢复和向量重建审计迁移
│   │   ├── 0007_viewer_merge.sql  # 身份合并墓碑和原始账本归属审计迁移
│   │   ├── 0008_memory_graph_invalidation.sql  # 记忆失效同事务生成关系墓碑与图同步任务的触发器
│   │   ├── 0009_superchat_room_enter_events.sql  # 扩展原始直播事件类型约束以支持醒目留言和进房事件
│   │   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── src/  # 按外部能力组织的适配器源码
│   │   ├── live/  # 直播源连接、事件标准化与模拟输入
│   │   │   ├── bilibili/  # 哔哩哔哩官方直播开放平台的授权、签名、连接与事件适配
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── config.rs  # 哔哩哔哩接入凭据、网络地址与资源上限配置校验
│   │   │   │   ├── connection.rs  # 官方直播长连接鉴权、接收、心跳与显式会话清理
│   │   │   │   ├── http.rs  # 开放平台签名 HTTP 项目开始、心跳、结束与错误处理
│   │   │   │   ├── mod.rs  # 哔哩哔哩直播适配器的模块与配置公开出口
│   │   │   │   ├── protocol.rs  # 官方直播二进制包解析、有界解压、鉴权回复与领域事件转换
│   │   │   │   └── signing.rs  # 开放平台请求体 MD5 与规范请求头 HMAC-SHA256 签名
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── mod.rs  # 直播源接入和事件标准化；去重、话题筛选与礼物合并属于 application。
│   │   │   └── simulator.rs  # 模拟弹幕、礼物和连接变化的事件来源，用于首个互动闭环与事件回放。
│   │   ├── llm/  # 云端及本地 LLM 的协议适配
│   │   │   ├── models/  # 模型目录配置及响应解析实现
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── config.rs  # 模型目录地址规范化与密钥校验
│   │   │   │   └── response.rs  # 供应商模型条目及分页元数据解析
│   │   │   ├── reasoning/  # 具体模型推理能力登记与原生参数映射
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   └── capabilities.rs  # 经官方文档核对的模型推理档位和预算预设能力表
│   │   │   ├── runtime/  # 四协议工具调用、流式响应、缓存和用量的统一适配
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── mod.rs  # 有界单轮模型调用与只读流式观测
│   │   │   │   ├── output.rs  # 原生输出、工具签名与用量归一化
│   │   │   │   ├── request.rs  # 原生工具、续接上下文与稳定缓存前缀请求
│   │   │   │   └── stream.rs  # 四协议 SSE 拼包、结束验证与用量保留
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── config.rs  # LLM 地址规范化、认证头及资源限制的统一校验
│   │   │   ├── mod.rs  # 模型协议适配。兼容同一协议的云端与本地服务复用实现，其他协议独立添加。
│   │   │   ├── models.rs  # 受限的供应商模型目录 HTTP 适配与分页汇总
│   │   │   ├── multi_provider.rs  # 多提供商决策与原生工具、流式运行层适配
│   │   │   ├── openai_compatible.rs  # 非流式 Chat Completions 传输、认证、响应大小与临时错误分类
│   │   │   ├── prompt.rs  # 跨 LLM 协议共享的事件回复与主动发言提示隔离及输入校验
│   │   │   ├── reasoning.rs  # 推理档位夹取、预算限制和各协议原生请求参数注入
│   │   │   └── response.rs  # LLM 文本决策内容与 OpenAI 聊天响应的严格校验
│   │   ├── speech/  # GPT-SoVITS 等语音引擎的请求和音频格式适配
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── gpt_sovits.rs  # GPT-SoVITS HTTP 适配入口：参考素材路径解析、合成参数映射与音频解码。
│   │   │   ├── mod.rs  # 语音引擎适配与音频格式转换。推理服务和模型权重位于项目目录之外。
│   │   │   ├── model_synthesizer.rs  # 模型内存启停、成对权重加载与合成互斥及取消事务保护
│   │   │   ├── resource_synthesizer.rs  # 根据音色档案解析参考音频并调用 GPT-SoVITS
│   │   │   └── wav.rs  # 完整 RIFF/WAV 边界与 PCM16 格式校验解码
│   │   ├── storage/  # PostgreSQL 观众事件与 Linux 素材文件存储
│   │   │   ├── postgres/  # 观众持久业务、记忆与关系的 PostgreSQL 实现
│   │   │   │   ├── memory/  # 记忆记录、后台租约和向量分层持久实现
│   │   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   │   ├── operations.rs  # 审计化任务恢复与向量重建事务
│   │   │   │   │   ├── queue.rs  # 记忆提取任务租约、重试及来源复核
│   │   │   │   │   ├── records.rs  # 记忆证据合并、时效检索和管理失效事务
│   │   │   │   │   └── vectors.rs  # 模型维度正文版本隔离的 pgvector 存取
│   │   │   │   ├── relationships/  # 关系图投影租约与恢复实现目录
│   │   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   │   ├── derived.rs  # 从可靠记忆派生明确兴趣、活动及未解析提及的事务规则
│   │   │   │   │   └── queue.rs  # 关系图事务发件箱认领重试和重建
│   │   │   │   ├── viewer_merge/  # 身份合并内部数据迁移实现
│   │   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   │   └── migrate.rs  # 合并身份时的去重迁移与账本对账
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── companionship.rs  # 原子来访礼物计分及幂等完成回执账本
│   │   │   │   ├── memory.rs  # 记忆持久端口实现与事务通用约束
│   │   │   │   ├── relationships.rs  # 权威关系事实、来源确认和版本化管理事务
│   │   │   │   └── viewer_merge.rs  # 有预览指纹与审计保护的身份合并事务
│   │   │   ├── resources/  # 资源快照转换与参考音频校验实现
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── snapshot.rs  # 资源快照版本化转换及音色参考标识一致性验证
│   │   │   │   └── wav.rs  # 参考音频 PCM 格式、时长和静音校验
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── files.rs  # Linux 素材文件存储，负责文件落盘及引擎可访问路径；Windows 模型导入另属 desktop-runtime。
│   │   │   ├── mod.rs  # 持久化和本地素材存储实现；应用层只看存取接口。
│   │   │   ├── postgres.rs  # 观众身份事件幂等接收与查询、迁移连接及业务存储集成
│   │   │   ├── receipt_journal.rs  # 同步持久、容量有界及公平重试的本地完成回执日志
│   │   │   ├── resources.rs  # 原子资源快照、音色与参考标识一致性校验及参考 WAV 存取清理
│   │   │   └── sqlite.rs  # SQLite 记录存储适配入口。表结构和迁移随首个持久化用例加入。
│   │   ├── training/  # 独立训练进程、进度及产物的适配
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── mod.rs  # 独立训练进程适配，处理进程状态与训练产物；调度策略属于 application。
│   │   │   ├── process.rs  # 独立训练进程组启动、超时取消和有界日志进度
│   │   │   ├── store.rs  # 任务素材与续训权重私有副本、原子历史存储和成对模型校验
│   │   │   ├── tests.rs  # 训练音色持久化、音频模式、转写进程、删除清理及存储故障测试
│   │   │   └── transcription.rs  # 单片音频自动转写的有界子进程、私有临时素材与退出清理
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── graph.rs  # Neo4j Query API 参数化投影和有界邻居检索
│   │   ├── lib.rs  # 外部能力实现：依赖业务层定义的 ports，不反向定义业务规则。
│   │   ├── memory.rs  # 独立 HTTP 记忆提取和嵌入服务适配器
│   │   ├── runtime.rs  # GPU 采样、WSL 工具查找与有界错误诊断
│   │   └── search.rs  # Brave 与 SearXNG 有界检索、来源过滤和错误脱敏
│   ├── tests/  # GPT-SoVITS、WAV 与 LLM 适配器的集成和输入输出边界测试
│   │   ├── bilibili_support/  # 受控官方直播 HTTP 和 WebSocket 协议测试服务
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── mod.rs  # 官方直播模拟响应、鉴权帧、事件和生命周期观测夹具
│   │   ├── llm_support/  # LLM 适配器测试公共夹具
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── mod.rs  # 受控 LLM HTTP 服务、配置、事件和完成响应夹具
│   │   ├── support/  # 语音适配测试的 WAV 和 HTTP 公共夹具
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── http.rs  # 受控 TTS HTTP 服务与请求配置夹具
│   │   │   ├── mod.rs  # 内存 WAV 生成与公共夹具模块导出
│   │   │   └── raw_http.rs  # 分块响应测试所用原始 HTTP 请求读取夹具
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── bilibili_connection.rs  # 官方直播源签名请求、长连接授权、事件与清理行为测试
│   │   ├── bilibili_messages.rs  # 直播消息包头、压缩边界、事件映射与无效输入测试
│   │   ├── bilibili_resilience.rs  # 直播心跳互不阻塞、鉴权同包事件、失败清理、事件标识和总解压预算回归测试
│   │   ├── bilibili_signing.rs  # 哔哩哔哩签名固定向量、请求体字节与输入验证测试
│   │   ├── gpt_sovits_http.rs  # GPT-SoVITS 参数映射、完整长 SC 音频响应和引擎失败测试
│   │   ├── gpt_sovits_limits.rs  # 引擎响应体上限、分块传输、总超时和重定向测试
│   │   ├── gpt_sovits_validation.rs  # 引擎配置、默认音色和播报文本输入校验测试
│   │   ├── llm_cancellation.rs  # 取消模型决策 future 后关闭在途 HTTP 连接测试
│   │   ├── llm_limits.rs  # 配置请求响应上限、总超时、错误分类与敏感信息脱敏测试
│   │   ├── llm_models.rs  # 模型目录认证、地址规范化和供应商分页集成测试
│   │   ├── llm_models_bounds.rs  # 模型目录资源上限、畸形响应及密钥保护边界测试
│   │   ├── llm_multi_provider.rs  # 多 LLM 协议的请求认证、输出校验和边界测试
│   │   ├── llm_output_validation.rs  # 严格决策字段、事件子集、工具调用、截断及内容约束测试
│   │   ├── llm_reasoning.rs  # 推理档位上下边界、缺档、默认与未知模型的解析回归测试
│   │   ├── llm_reasoning_http.rs  # 推理参数在原生 HTTP、流式和工具续传中的适配与上限测试
│   │   ├── llm_runtime.rs  # 四协议运行层、工具续接、缓存及流式取消回归测试
│   │   ├── llm_transport.rs  # 路径认证、消息角色、主动发言历史隔离、礼物分组提示与重定向测试
│   │   ├── memory_http.rs  # 提取嵌入 HTTP 格式限额与来源校验测试
│   │   ├── model_runtime.rs  # 模型关闭拒绝合成、启停后恢复及不污染权重状态的 HTTP 测试
│   │   ├── model_synthesizer.rs  # 取消后的权重合成互斥与默认模型恢复链路测试
│   │   ├── neo4j_graph.rs  # 真实 Neo4j 版本墓碑范围隔离及恢复测试
│   │   ├── postgres_companionship.rs  # 真实 PostgreSQL 陪伴日预算并发幂等测试
│   │   ├── postgres_derived_relations.rs  # 真实 PostgreSQL 记忆派生关系与删除撤销不复活测试
│   │   ├── postgres_memories.rs  # 真实 PostgreSQL 记忆生命周期租约和管理抑制测试
│   │   ├── postgres_relationships.rs  # 真实 PostgreSQL 关系证据图发件箱与隔离测试
│   │   ├── postgres_viewer_merge.rs  # 真实 PostgreSQL 身份合并预览去重和陈旧请求测试
│   │   ├── postgres_viewers.rs  # PostgreSQL 观众身份、事件去重、并发、范围隔离和批次原子性容器测试
│   │   ├── receipt_journal.rs  # 回执日志重启幂等、文件权限、容量和损坏诊断测试
│   │   ├── resource_store.rs  # 参考音频存储持久化、删除重启、路径及标识一致性测试
│   │   ├── resource_synthesizer.rs  # 上传音色的引擎路径解析与默认音色回退测试
│   │   ├── wav_decoding.rs  # 完整 WAV 的基础 PCM 解码与无效输入测试
│   │   ├── wav_validation.rs  # WAV 采样率、位深、帧完整性和容器畸形校验测试
│   │   └── web_search.rs  # 搜索请求格式、来源过滤及响应长度边界测试
│   ├── Cargo.toml  # 该 Rust 包的名称、workspace 配置与模块依赖声明
│   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── application/  # 业务用例编排及外部能力接口定义
│   ├── src/  # Agent、事件调度、语音与资源任务用例
│   │   ├── agent/  # Agent 配置、输出校验、播放关联及状态类型
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── decisions.rs  # 模型输出校验、时效与繁忙复核、工作隔离和完整原文语音准备
│   │   │   ├── interaction.rs  # 有界流量统计、弹幕朗读模式、进房欢迎冷却及原文播报前缀
│   │   │   ├── playback.rs  # 播放状态同步和已完成对话记忆
│   │   │   ├── settings.rs  # 人设配置和调度资源上限校验
│   │   │   └── types.rs  # Agent 阶段、事件状态及应用调用结果
│   │   ├── ports/  # 业务方定义的模型、语音、存储和执行能力边界
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── companionship.rs  # 陪伴账本礼物确认与完成回执存储接口
│   │   │   ├── execution.rs  # Windows 执行指令下发、取消和执行回执接收的能力边界；实现不在业务层。
│   │   │   ├── live_source.rs  # 平台无关直播源、连接生命周期和错误语义接口
│   │   │   ├── llm.rs  # 模型决策、已完成对话与可取消异步模型接口
│   │   │   ├── llm_runtime.rs  # 统一模型工具轮次、流式观测与 token 用量接口
│   │   │   ├── memory.rs  # 记忆提取与向量嵌入能力接口
│   │   │   ├── memory_store.rs  # 权威记忆、后台任务及管理恢复存储接口
│   │   │   ├── mod.rs  # 由业务方定义的外部能力接口。实现位于 adapters 或应用入口的传输适配层。
│   │   │   ├── reasoning.rs  # 厂商无关的八档推理强度顺序、默认值与严格解析
│   │   │   ├── receipt_journal.rs  # 播放完成回执本地持久暂存与数据库提交确认接口
│   │   │   ├── relationships.rs  # 关系事实、图投影和同步恢复能力接口
│   │   │   ├── speech.rs  # 可动态注入的异步语音合成接口与 PCM 输出类型
│   │   │   ├── storage.rs  # 资源快照、参考音频与引擎路径存储接口
│   │   │   ├── training.rs  # 训练存储、同音色续训基底与受控进程接口
│   │   │   ├── viewer_merge.rs  # 身份合并预览及版本条件应用接口
│   │   │   ├── viewers.rs  # 观众身份和直播事件的幂等持久接收及分页查询端口
│   │   │   └── web_search.rs  # 只读网页搜索结果与异步查询业务接口
│   │   ├── scheduler/  # 候选事件优先级与礼物分组策略
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── fairness.rs  # 完成驱动的观众公平、有限重选和追问焦点
│   │   │   └── selection.rs  # SC 独立优先选择、欢迎单轮隔离、朗读长度约束与礼物分组
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── agent.rs  # Agent 生命周期、决策调度和状态快照
│   │   ├── lib.rs  # 业务用例与外部能力接口。通过注入 ports 的实现调用外部能力。
│   │   ├── performance.rs  # 协调发言、动作、下发与执行回执；处理代次、取消及重连后未知状态。
│   │   ├── resources.rs  # 角色与音色档案用例、删除和选择清理、持久化事务及映射一致性
│   │   ├── scheduler.rs  # 有界事件接收、去重、优先级与保留源事件剩余时效的调度
│   │   ├── session.rs  # 会话启动、暂停、恢复与关闭用例，协调在途任务的生命周期。
│   │   ├── speech.rs  # 单执行者语音 FIFO 队列、容量历史限制与取消回执状态机
│   │   └── training.rs  # 训练任务调度、同音色续训基底选择及版本保存选用
│   ├── tests/  # 应用用例的队列与状态流转集成测试
│   │   ├── agent_support/  # Agent 测试公共夹具
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── mod.rs  # 事件、决策、会话和播放任务测试构造
│   │   ├── support/  # 应用层测试共用队列夹具
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── mod.rs  # 已连接队列、下发和完成流程测试夹具
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── agent_decisions.rs  # 输出校验、暂停隔离及决策冷却测试
│   │   ├── agent_fairness.rs  # 稳定身份公平调度与完成反馈边界测试
│   │   ├── agent_lifecycle.rs  # 配置、暂停、停止和播放生命周期测试
│   │   ├── agent_memory.rs  # 已完成对话数量及内容长度边界测试
│   │   ├── agent_settings.rs  # 人设配置与运行资源上限测试
│   │   ├── interaction_policy.rs  # SC 优先和时效、弹幕流量策略、欢迎抑制冷却及长原文播报测试
│   │   ├── resource_library.rs  # 资源事务失败保护、角色音色删除绑定及清理重试用例测试
│   │   ├── scheduler_bounds.rs  # 历史裁剪、去重淘汰、批次容量和克隆隔离测试
│   │   ├── scheduler_events.rs  # 事件去重、过期、容量及礼物分组测试
│   │   ├── speech_cancellation.rs  # 语音停止代次、迟到结果和断线取消测试
│   │   ├── speech_queue.rs  # 语音队列 FIFO、容量、接收门控和终态历史测试
│   │   └── speech_receipts.rs  # 执行回执顺序、重复回执与失败状态测试
│   ├── Cargo.toml  # 该 Rust 包的名称、workspace 配置与模块依赖声明
│   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── desktop-runtime/  # 独立于界面的 Windows 播放与设备执行库
│   ├── src/  # 播放、连接、口型、VTS、OBS 与模型导入的执行源码
│   │   ├── assets/  # Live2D 导出模型包的本地校验与安装
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── model.rs  # 模型清单与路径校验、无覆盖安装、模型身份枚举及持久化删除重试
│   │   ├── audio/  # 音频设备后端、采样转换与设备播放时钟
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── buffer.rs  # 跨平台重采样输出缓冲，按需分配并限制实际排队 PCM 为 128 MiB，附长 SC 和溢出测试
│   │   │   ├── conversion.rs  # 相位连续的 PCM 重采样与声道映射
│   │   │   ├── meter.rs  # 固定容量的设备播放能量时间线与 RMS 累加器
│   │   │   ├── simulated.rs  # 显式静音模拟后端，按模拟播放时间退役样本并观测能量
│   │   │   ├── timing.rs  # 设备计划播放时间与完成回执时钟
│   │   │   └── windows.rs  # Windows 默认输出设备、受限按需重采样缓冲及真实输出能量和时钟回执
│   │   ├── avatar/  # VTube Studio 私有协议、配置校验、授权存储与口型连接状态机
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── client.rs  # 有界 VTS WebSocket 请求响应、请求关联、API 错误和口型参数注入
│   │   │   ├── config.rs  # VTS 默认关闭配置与地址、参数名称、超时范围校验
│   │   │   ├── resources.rs  # VTS 已授权模型查询加载与热键核对预览
│   │   │   ├── switching.rs  # 切换角色口型输入并等待 VTS 应用确认
│   │   │   ├── token.rs  # 阻塞线程池中的私有授权令牌读写、权限校验及可取消等待
│   │   │   └── worker.rs  # 最新口型值消费、授权复用、归零、样本过期与自动重连状态机
│   │   ├── bin/  # 独立桌面执行客户端命令入口
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── meowlive-client.rs  # 独立桌面执行客户端程序入口
│   │   │   └── meowlive-model.rs  # 独立模型包校验与显式目录安装命令行
│   │   ├── lip_sync/  # 设备能量到口型的纯规则、配置与音频后端包装器
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── backend.rs  # 音频后端观察包装器，定时发布最新口型并在终态同步复位
│   │   │   ├── config.rs  # 口型阈值、增益、平滑时间和更新频率的默认值与校验
│   │   │   └── envelope.rs  # 按经过时间执行开闭口平滑与静音复位的纯计算器
│   │   ├── obs/  # OBS 本地连接配置与校验
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── config.rs  # OBS 本机 WebSocket 连接、兼容环境变量和私有设置路径校验
│   │   │   └── settings.rs  # 执行端 OBS 设置私有文件读写、凭据保留清除及公开状态脱敏
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── assets.rs  # 本机 Live2D 模型包校验、安装、枚举及删除能力导出
│   │   ├── audio.rs  # 音频设备边界、播放事件与设备输出能量观测接口
│   │   ├── avatar.rs  # VTube Studio 独立连接入口、配置与可观察状态导出
│   │   ├── cli.rs  # 桌面客户端配置加载、服务重连、VTS 组装与限时退出入口
│   │   ├── config.rs  # 执行客户端连接、缓冲、VTS 与口型配置校验及路径解析
│   │   ├── connection.rs  # 携带独立设备凭据连接主服务、接收控制音频及回传播放回执
│   │   ├── host.rs  # Tauri 与 CLI 共用的可取消执行宿主及退出清理
│   │   ├── lib.rs  # Windows 执行库：与 Tauri 和 React 解耦，生命周期由桌面进程管理。
│   │   ├── lip_sync.rs  # 设备输出口型的配置、平滑器与观察后端导出
│   │   ├── obs.rs  # 使用本机面板设置的 OBS WebSocket v5 鉴权、场景切换及录制控制
│   │   ├── playback.rs  # 播放状态机、代次取消、乱序校验与设备回执
│   │   ├── presentation.rs  # 桌面口型驱动生命周期与角色参数切换组装
│   │   └── resource_control.rs  # 桌面模型导入列举删除、VTS 加载热键与 OBS 控制执行
│   ├── tests/  # 桌面执行运行时独立集成测试
│   │   ├── avatar_support/  # VTS 本地 WebSocket 场景测试的隔离文件与协议辅助设施
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── mod.rs  # 受控 VTS 服务器、授权交互与参数断言共用辅助
│   │   ├── connection/  # 使用虚拟时间和真实 WebSocket 的连接期限测试
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── flush.rs  # 阻塞 Pong 写回的超时测试
│   │   │   └── liveness.rs  # 双通道独立心跳期限、续期及设备清理测试
│   │   ├── lip_sync_support/  # 口型观察测试的共享可控设备夹具
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── mod.rs  # 可设置能量、事件与操作错误的测试音频设备
│   │   ├── support/  # 运行时测试共用手动设备与消息夹具
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── mod.rs  # 可控设备事件及播报消息测试夹具
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── avatar_authorization.rs  # VTS 首次授权、令牌复用、拒绝撤销、授权中退出与断线测试
│   │   ├── avatar_cli.rs  # 真实客户端进程的 VTS 连接、配置加载与断线归零联调
│   │   ├── avatar_config_validation.rs  # VTS 默认关闭、未知键与配置边界校验测试
│   │   ├── avatar_configuration.rs  # 客户端 VTS 和口型配置的兼容、校验与相对路径测试
│   │   ├── avatar_lifecycle.rs  # VTS 表现输出组装在禁用与等待授权时的关闭测试
│   │   ├── avatar_parameters.rs  # VTS 最新值映射、参数范围、注入心跳、限频和过期归零测试
│   │   ├── avatar_protocol.rs  # VTS 响应类型与请求关联校验、总超时和错误信息脱敏测试
│   │   ├── avatar_reconnect.rs  # VTS 重连先归零、服务不可用和未响应请求中的退出测试
│   │   ├── avatar_resources.rs  # VTS 模型与热键请求和错配边界测试
│   │   ├── avatar_stop.rs  # VTS 请求未回复时主服务停止回执与口型归零的隔离测试
│   │   ├── avatar_switching.rs  # 角色口型切换归零、应用确认与禁用驱动测试
│   │   ├── avatar_token_storage.rs  # VTS 本地令牌长度、字符、文件权限、符号链接和磁盘等待取消测试
│   │   ├── client_cli.rs  # 客户端帮助与缺失配置退出行为测试
│   │   ├── client_configuration.rs  # TOML、配对 URL 与平台设备边界测试
│   │   ├── device_auth.rs  # 私有设备凭据在控制和音频 WebSocket 握手中传递的集成测试
│   │   ├── device_conversion.rs  # PCM 声道映射与跨分片重采样测试
│   │   ├── device_timing.rs  # 设备延迟、播放开始与完成时钟测试
│   │   ├── host_lifecycle.rs  # 宿主启动失败、离线取消和双通道退出清理测试
│   │   ├── lip_sync_delivery.rs  # 口型发布频率、静音保鲜、最新值覆盖与停止抢占测试
│   │   ├── lip_sync_failures.rs  # 设备启动、写入和结束失败时的停止与口型复位测试
│   │   ├── lip_sync_levels.rs  # 口型能量阈值、增益、时间平滑及非法输入测试
│   │   ├── lip_sync_lifecycle.rs  # 设备驱动口型、停止、完成、失败和析构复位测试
│   │   ├── long_speech.rs  # 默认桌面缓冲完整接收四分钟 SC 音频分片及完成回执边界测试
│   │   ├── model_assets.rs  # 模型引用、路径限制、安装与覆盖保护测试
│   │   ├── model_management.rs  # 已安装 Live2D 模型列表、删除、路径边界及跨端协议测试
│   │   ├── obs_configuration.rs  # OBS 地址校验、本机保存与重启读取、密码保留清除及文件权限测试
│   │   ├── obs_websocket.rs  # OBS 受控鉴权、本机密码热读取、状态读回与禁止重放写请求测试
│   │   ├── output_meter.rs  # 设备能量的播放延迟、静音、过期、容量与复位测试
│   │   ├── playback_completion.rs  # 播放完成与设备失败回执测试
│   │   ├── playback_ordering.rs  # 控制音频竞态、乱序及有界待播缓存测试
│   │   ├── playback_stop.rs  # 停止代次、迟到音频及断线清理测试
│   │   ├── simulated_levels.rs  # 模拟播放电平的时间位置、停止、完成与缓冲上限测试
│   │   ├── websocket_disconnect.rs  # 音频连接中断时停止设备与关闭控制连接测试
│   │   └── websocket_session.rs  # 双 WebSocket 配对与实际客户端回执闭环测试
│   ├── Cargo.toml  # 该 Rust 包的名称、workspace 配置与模块依赖声明
│   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── domain/  # 业务对象、状态和不变量，保持无外部依赖
│   ├── src/  # 事件、会话、角色、音色和执行状态的领域定义
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── affinity.rs  # 整数毫分、每日礼物收益及分值饱和纯规则
│   │   ├── character.rs  # 角色能力映射、验证状态与口型参数规则
│   │   ├── event.rs  # 聊天、礼物、SC 和进房领域事件、稳定身份、输入校验与响应时效
│   │   ├── lib.rs  # 业务领域：只表达业务对象、状态和不变量，不依赖通信、框架或外部服务。
│   │   ├── memory.rs  # 有来源证据的记忆晋升、期限和时效纯规则
│   │   ├── performance.rs  # 发言与角色动作的执行意图和状态；播放回执决定实际执行结果。
│   │   ├── relationships.rs  # 关系实体、事实类型及确认等级领域类型
│   │   ├── resources.rs  # 音色、参考素材、资源目录及纯业务不变量
│   │   ├── session.rs  # 直播会话的状态与合法状态转换；与每条发言的生命周期分别建模。
│   │   ├── speech.rs  # 人工与组合播报的文本边界、任务快照、生成代次与执行状态语义
│   │   ├── training.rs  # 训练参数、性能边界、片段校验和任务状态领域规则
│   │   └── voice.rs  # 配置音色标识校验
│   ├── tests/  # 领域对象与输入不变量集成测试
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── affinity.rs  # 礼物收益增量和分值饱和边界测试
│   │   ├── event_validation.rs  # 事件身份与内容、礼物数量、SC 金额时效及元数据归属校验测试
│   │   ├── memory.rs  # 明确自述双日证据、敏感候选及记忆期限测试
│   │   ├── resource_validation.rs  # 资源名称、语言、素材元数据与能力映射校验测试
│   │   └── speech_validation.rs  # 人工与组合播报文本 Unicode 长度、字符和音色标识校验测试
│   ├── Cargo.toml  # 该 Rust 包的名称、workspace 配置与模块依赖声明
│   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── protocol/  # 跨进程控制、事件、音频和执行消息的契约源
│   ├── src/  # 与业务领域分离的通信 DTO 模块
│   │   ├── bin/  # 协议开发命令入口
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── export-types.rs  # 从 Rust DTO 生成并检查 TypeScript 契约
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── agent.rs  # Agent 设置、直播事件、阶段、处理状态与公开快照 DTO
│   │   ├── audio.rs  # PCM 格式约定及有界二进制音频帧编解码
│   │   ├── auth.rs  # 管理员认证状态、登录请求和短期会话响应契约
│   │   ├── companionship.rs  # 仅供管理端的陪伴账本与礼物确认契约
│   │   ├── control.rs  # 文字播报请求、状态快照与桌面控制消息契约
│   │   ├── event.rs  # 提供给控制面板的状态通知和事件封装；不透传平台私有事件。
│   │   ├── execution.rs  # 桌面执行状态与播放回执契约
│   │   ├── launcher.rs  # 本机主服务、TTS 和 Windows 执行端三开关的启动管理契约
│   │   ├── lib.rs  # 跨进程通信契约的唯一来源。与业务领域对象分离，按协议版本演进。
│   │   ├── live.rs  # 直播平台连接状态、面板凭据配置请求与脱敏快照契约
│   │   ├── llm.rs  # LLM 接入配置、模型目录与统一推理档位预览的跨端契约
│   │   ├── llm_runtime.rs  # 运行配置、模型单价、调用用量与活动跨端契约
│   │   ├── memory.rs  # 记忆证据管理和后台任务状态跨端契约
│   │   ├── model_library.rs  # 本机环境、模型目录、安装结果及下载任务的跨进程契约
│   │   ├── obs.rs  # OBS 场景录制操作、本机连接设置与脱敏状态契约
│   │   ├── relationships.rs  # 关系事实管理和图同步状态跨端契约
│   │   ├── resources.rs  # 角色音色档案及桌面模型、OBS 操作和私有设置桥接契约
│   │   ├── training.rs  # 训练片段、性能参数、任务版本及离线测量跨端契约
│   │   ├── training_runtime.rs  # 独立于 TTS 服务的模型内存启停请求与状态契约
│   │   ├── viewer_merge.rs  # 身份合并预览和明确确认的跨端契约
│   │   └── viewers.rs  # 管理员观众身份、昵称历史和持久事件分页契约
│   ├── tests/  # 通信协议独立集成测试
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── agent_contracts.rs  # Agent 输入严格反序列化及公开 JSON 形状测试
│   │   ├── audio_frames.rs  # PCM 二进制帧编码、边界与损坏输入测试
│   │   ├── compatibility.rs  # 协议必填字段、训练模式缺省兼容、未知标签与音频格式测试
│   │   ├── control_serialization.rs  # 控制消息与执行回执序列化测试
│   │   └── obs_contracts.rs  # OBS 指令及设置请求未知字段拒绝、凭据脱敏与桌面资源契约测试
│   ├── Cargo.toml  # 该 Rust 包的名称、workspace 配置与模块依赖声明
│   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
└── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
```

可继续查看各子目录的索引：

- [adapters/](adapters/DIRECTORY.md)：外部服务和存储实现，适配 application 定义的能力
- [application/](application/DIRECTORY.md)：业务用例编排及外部能力接口定义
- [desktop-runtime/](desktop-runtime/DIRECTORY.md)：独立于界面的 Windows 播放与设备执行库
- [domain/](domain/DIRECTORY.md)：业务对象、状态和不变量，保持无外部依赖
- [protocol/](protocol/DIRECTORY.md)：跨进程控制、事件、音频和执行消息的契约源

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: f6bd0ab501c3d80169f7b03f051489ae32c257698984275dd8ab03d775c4b63d -->
