# adapters 目录索引

外部服务和存储实现，适配 application 定义的能力

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
adapters/  # 外部服务和存储实现，适配 application 定义的能力
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
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── config.rs  # LLM 地址规范化、认证头及资源限制的统一校验
│   │   ├── mod.rs  # 模型协议适配。兼容同一协议的云端与本地服务复用实现，其他协议独立添加。
│   │   ├── multi_provider.rs  # OpenAI Responses、Anthropic Messages、Gemini 与兼容聊天的协议适配
│   │   ├── openai_compatible.rs  # 非流式 Chat Completions 传输、认证、响应大小与临时错误分类
│   │   ├── prompt.rs  # 跨 LLM 协议共享的结构化 Agent 提示与输入校验
│   │   └── response.rs  # LLM 文本决策内容与 OpenAI 聊天响应的严格校验
│   ├── speech/  # GPT-SoVITS 等语音引擎的请求和音频格式适配
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── gpt_sovits.rs  # GPT-SoVITS HTTP 适配入口：参考素材路径解析、合成参数映射与音频解码。
│   │   ├── mod.rs  # 语音引擎适配与音频格式转换。推理服务和模型权重位于项目目录之外。
│   │   ├── model_synthesizer.rs  # 模型内存启停、成对权重加载与合成互斥及取消事务保护
│   │   ├── resource_synthesizer.rs  # 根据音色档案解析参考音频并调用 GPT-SoVITS
│   │   └── wav.rs  # 完整 RIFF/WAV 边界与 PCM16 格式校验解码
│   ├── storage/  # SQLite 记录与 Linux 素材文件存储
│   │   ├── resources/  # 资源快照转换与参考音频校验实现
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── snapshot.rs  # 资源快照版本化转换及音色参考标识一致性验证
│   │   │   └── wav.rs  # 参考音频 PCM 格式、时长和静音校验
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── files.rs  # Linux 素材文件存储，负责文件落盘及引擎可访问路径；Windows 模型导入另属 desktop-runtime。
│   │   ├── mod.rs  # 持久化和本地素材存储实现；应用层只看存取接口。
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
│   ├── lib.rs  # 外部能力实现：依赖业务层定义的 ports，不反向定义业务规则。
│   └── runtime.rs  # GPU 采样、WSL 工具查找与有界错误诊断
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
│   ├── gpt_sovits_http.rs  # GPT-SoVITS 请求参数映射、音频响应和引擎失败测试
│   ├── gpt_sovits_limits.rs  # 引擎响应体上限、分块传输、总超时和重定向测试
│   ├── gpt_sovits_validation.rs  # 引擎配置、默认音色和播报文本输入校验测试
│   ├── llm_cancellation.rs  # 取消模型决策 future 后关闭在途 HTTP 连接测试
│   ├── llm_limits.rs  # 配置请求响应上限、总超时、错误分类与敏感信息脱敏测试
│   ├── llm_multi_provider.rs  # 多 LLM 协议的请求认证、输出校验和边界测试
│   ├── llm_output_validation.rs  # 严格决策字段、事件子集、工具调用、截断及内容约束测试
│   ├── llm_transport.rs  # 路径、认证、JSON 模式、消息角色、礼物分组提示与重定向测试
│   ├── model_runtime.rs  # 模型关闭拒绝合成、启停后恢复及不污染权重状态的 HTTP 测试
│   ├── model_synthesizer.rs  # 取消后的权重合成互斥与默认模型恢复链路测试
│   ├── resource_store.rs  # 参考音频存储持久化、删除重启、路径及标识一致性测试
│   ├── resource_synthesizer.rs  # 上传音色的引擎路径解析与默认音色回退测试
│   ├── wav_decoding.rs  # 完整 WAV 的基础 PCM 解码与无效输入测试
│   └── wav_validation.rs  # WAV 采样率、位深、帧完整性和容器畸形校验测试
├── Cargo.toml  # 该 Rust 包的名称、workspace 配置与模块依赖声明
└── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
```

可继续查看各子目录的索引：

- [src/](src/DIRECTORY.md)：按外部能力组织的适配器源码
- [tests/](tests/DIRECTORY.md)：GPT-SoVITS、WAV 与 LLM 适配器的集成和输入输出边界测试

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 859b222dca4970312a4e5a6af099f4df164300d63437de1a6c98aa36b71567c8 -->
