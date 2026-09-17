# tests 目录索引

GPT-SoVITS、WAV 与 LLM 适配器的集成和输入输出边界测试

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
tests/  # GPT-SoVITS、WAV 与 LLM 适配器的集成和输入输出边界测试
├── bilibili_support/  # 受控官方直播 HTTP 和 WebSocket 协议测试服务
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── mod.rs  # 官方直播模拟响应、鉴权帧、事件和生命周期观测夹具
├── llm_support/  # LLM 适配器测试公共夹具
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── mod.rs  # 受控 LLM HTTP 服务、配置、事件和完成响应夹具
├── support/  # 语音适配测试的 WAV 和 HTTP 公共夹具
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── http.rs  # 受控 TTS HTTP 服务与请求配置夹具
│   ├── mod.rs  # 内存 WAV 生成与公共夹具模块导出
│   └── raw_http.rs  # 分块响应测试所用原始 HTTP 请求读取夹具
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── bilibili_connection.rs  # 官方直播源签名请求、长连接授权、事件与清理行为测试
├── bilibili_messages.rs  # 直播消息包头、压缩边界、事件映射与无效输入测试
├── bilibili_resilience.rs  # 直播心跳互不阻塞、鉴权同包事件、失败清理、事件标识和总解压预算回归测试
├── bilibili_signing.rs  # 哔哩哔哩签名固定向量、请求体字节与输入验证测试
├── gpt_sovits_http.rs  # GPT-SoVITS 请求参数映射、音频响应和引擎失败测试
├── gpt_sovits_limits.rs  # 引擎响应体上限、分块传输、总超时和重定向测试
├── gpt_sovits_validation.rs  # 引擎配置、默认音色和播报文本输入校验测试
├── llm_cancellation.rs  # 取消模型决策 future 后关闭在途 HTTP 连接测试
├── llm_limits.rs  # 配置请求响应上限、总超时、错误分类与敏感信息脱敏测试
├── llm_multi_provider.rs  # 多 LLM 协议的请求认证、输出校验和边界测试
├── llm_output_validation.rs  # 严格决策字段、事件子集、工具调用、截断及内容约束测试
├── llm_transport.rs  # 路径、认证、JSON 模式、消息角色、礼物分组提示与重定向测试
├── model_runtime.rs  # 模型关闭拒绝合成、启停后恢复及不污染权重状态的 HTTP 测试
├── model_synthesizer.rs  # 取消后的权重合成互斥与默认模型恢复链路测试
├── resource_store.rs  # 参考音频存储持久化、删除重启、路径及标识一致性测试
├── resource_synthesizer.rs  # 上传音色的引擎路径解析与默认音色回退测试
├── wav_decoding.rs  # 完整 WAV 的基础 PCM 解码与无效输入测试
└── wav_validation.rs  # WAV 采样率、位深、帧完整性和容器畸形校验测试
```

可继续查看各子目录的索引：

- [bilibili_support/](bilibili_support/DIRECTORY.md)：受控官方直播 HTTP 和 WebSocket 协议测试服务
- [llm_support/](llm_support/DIRECTORY.md)：LLM 适配器测试公共夹具
- [support/](support/DIRECTORY.md)：语音适配测试的 WAV 和 HTTP 公共夹具

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 1270c0c33061f1d864cb99ce4bddf7688587f3cd04cfcb892935ca99fe17efe3 -->
