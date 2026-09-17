# server 目录索引

Linux / WSL 主服务入口、配置和传输边界

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
server/  # Linux / WSL 主服务入口、配置和传输边界
├── src/  # 主服务启动、配置解析及 HTTP / WebSocket 适配源码
│   ├── agent/  # Agent 服务状态、跨端映射和异步模型调度
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── mapping.rs  # 统一事件及 Agent 业务状态到公开 HTTP DTO 的映射
│   │   ├── state.rs  # Agent 查询控制、原子事件接收及语音状态同步
│   │   └── worker.rs  # 锁外模型调用、有界重试、GPU 在途互斥及取消核对后的语音入队
│   ├── config/  # 按 Agent 与模型能力拆分的配置校验
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── agent.rs  # Agent 人设和有界调度参数的 TOML 配置
│   │   ├── live.rs  # 官方直播接入开关、应用编号、凭据变量名和重连参数校验
│   │   ├── llm.rs  # LLM 提供商协议、地址、模型及资源限额校验与密钥脱敏
│   │   ├── resources.rs  # Linux 资源保存目录和引擎共享挂载配置
│   │   └── training.rs  # 训练路径、本地识别模型、超时及受管推理默认权重配置校验
│   ├── live/  # 官方直播源组装及异步连接、接收、清理与重连驱动
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── bootstrap.rs  # 从主服务环境变量组装官方直播适配器
│   │   └── worker.rs  # 直播事件接收、去重入队、取消、会话清理和退避重连
│   ├── transport/  # 控制接口、跨端连接与协议到领域对象的转换
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── agent.rs  # Agent 状态控制与批量事件输入的 HTTP 边界
│   │   ├── bridge.rs  # 独立控制与音频发送循环、心跳及执行回执接收
│   │   ├── error.rs  # 稳定的结构化 HTTP 错误映射
│   │   ├── http.rs  # 组装播报、Agent、事件、直播连接及 WebSocket 路由和来源校验
│   │   ├── live.rs  # 直播连接查询、连接及断开的 HTTP 输入边界
│   │   ├── llm.rs  # LLM 接入配置读写与草稿连接测试 HTTP 入口
│   │   ├── mapping.rs  # protocol DTO 与 domain 类型的显式转换，避免序列化字段影响领域规则。
│   │   ├── mod.rs  # HTTP / WebSocket 输入与输出适配；在协议 DTO 与领域对象之间进行映射。
│   │   ├── obs.rs  # 通过桌面资源通道执行 OBS 状态查询与受限控制
│   │   ├── origin.rs  # 浏览器请求来源校验及 HTTP 来源中间件
│   │   ├── resources.rs  # 音色和角色增删选择、模型删除代理及能力验证路由
│   │   ├── runtime.rs  # 离线推理测量、GPU 采样错误传播与预设查询
│   │   ├── training.rs  # 训练素材导入、性能设置、按所选 GPU 准入及版本试听保存接口
│   │   ├── training_models.rs  # 模型内存状态查询及带资源互斥和取消保护的启停接口
│   │   └── websocket.rs  # 唯一执行端连接准入及音频配对校验
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── agent.rs  # Agent 异步驱动与服务状态组装入口
│   ├── bootstrap.rs  # 读取私有配置，组装语音、LLM 和直播适配器并管理服务生命周期
│   ├── config.rs  # TOML 配置、本机 LLM 覆盖加载及启动前全局校验
│   ├── gpu.rs  # 训练试听测量独占租约、保留直播及 Agent 状态的音色切换准入与互斥测试
│   ├── lib.rs  # 可注入适配器的服务模块导出与集成测试入口
│   ├── live.rs  # 直播连接单会话所有权、公开快照和手动连接断开控制
│   ├── llm_settings.rs  # 本机 LLM 覆盖配置的原子保存、凭据隔离与重启状态查询
│   ├── main.rs  # Linux / WSL 主服务入口。业务编排位于 meowlive-application。
│   ├── resources.rs  # 桌面资源请求关联、单操作准入与取消生命周期
│   ├── state.rs  # 语音、Agent、唯一执行桥接和直播连接的共享状态及取消生命周期
│   └── worker.rs  # 语音任务 I/O 驱动、PCM 分片传输与设备回执等待
├── tests/  # 主服务配置、HTTP、桥接与完整播报集成测试
│   ├── agent_support/  # Agent 服务集成测试公共夹具
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   └── mod.rs  # 可注入模型与语音的服务和后台任务测试组装
│   ├── live_support/  # 可控直播源与事件的服务生命周期测试支持
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   └── mod.rs  # 可控直播连接、会话队列、事件与状态等待工具
│   ├── process_support/  # 真实主服务进程测试的隔离目录和音频构造辅助
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   └── mod.rs  # 主服务测试子进程启动清理和受控 WAV 构造
│   ├── support/  # 主服务集成测试公共夹具
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   └── mod.rs  # 受控合成器、HTTP 请求与双 WebSocket 连接夹具
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── agent_api.rs  # Agent 默认暂停、设置、事件校验、去重与停止接口测试
│   ├── agent_bootstrap.rs  # LLM 启动配置、缺失环境变量及无认证本地模型测试
│   ├── agent_cancellation.rs  # 暂停停止配置修改和断线取消在途 LLM 的集成测试
│   ├── agent_capacity.rs  # 事件批量请求大小和容量拒绝的原子性测试
│   ├── agent_configuration.rs  # Agent 和 LLM 的默认配置、字段边界及解析错误脱敏测试
│   ├── agent_process.rs  # 真实主服务进程、受控 LLM 与 TTS、静音客户端的播报闭环测试
│   ├── agent_receipt_retention.rs  # 语音历史裁剪时保留 Agent 已完成播放结果的回归测试
│   ├── agent_retries.rs  # 模型临时错误分类和有限重试次数的集成测试
│   ├── agent_runtime.rs  # 模拟事件到设备播放完成及断线未知状态的集成测试
│   ├── bridge_handshake.rs  # 协议版本、唯一执行端与双连接配对测试
│   ├── configuration.rs  # 示例配置兼容及无效参数拒绝测试
│   ├── http_api.rs  # 状态、播报输入与停止接口测试
│   ├── live_admission.rs  # 直播事件容量丢弃、无效事件及退出清理测试
│   ├── live_cleanup.rs  # 直播延迟清理、清理失败、房间状态与退避取消回归测试
│   ├── live_configuration.rs  # 直播凭据缺失、环境变量边界与配置验证测试
│   ├── live_http.rs  # 直播默认禁用状态、连接入口和配置范围测试
│   ├── live_lifecycle.rs  # 单直播连接、取消迟到结果、事件去重与重连终止测试
│   ├── live_process.rs  # 真实主服务进程的直播凭据组装、默认不连接与公开响应脱敏测试
│   ├── live_runtime.rs  # 受控平台礼物与重复帧经真实适配器、Agent、语音和静音设备完成回执的联调测试
│   ├── llm_profile.rs  # LLM 协议组装、私有配置持久化、重启加载及 HTTP 验证
│   ├── m5_config.rs  # 本地预设地址资源上限及训练配置约束测试
│   ├── obs_http.rs  # OBS HTTP 参数、桌面连接要求及模拟执行端贯通测试
│   ├── request_origin.rs  # HTTP 来源拒绝和无副作用保障测试
│   ├── resources_bridge.rs  # 桌面资源关联、模型删除引用与并发保护、断连响应边界测试
│   ├── resources_characters.rs  # 角色加载确认、预览验证及映射变更失效测试
│   ├── resources_http.rs  # 初始空音色、角色删除、非法资源及音色别名 HTTP 测试
│   ├── resources_process.rs  # 真实服务进程上传恢复试听、Agent、资源删除持久化链路测试
│   ├── resources_runtime.rs  # 面板经真实桌面执行库到受控 VTS 的角色、热键与停止链路测试
│   ├── runtime_loop.rs  # 真实服务与独立桌面运行时的静音播放集成测试
│   ├── speech_delivery.rs  # PCM 下发、设备回执门控及独立停止通道测试
│   ├── synthesis_cancellation.rs  # 合成停止、断线未知与重连不重播集成测试
│   ├── training_http.rs  # 训练配置、音色版本删除与当前选择清理、存储故障的 HTTP 测试
│   ├── training_models.rs  # 受控 HTTP 验证模型开关及无需开启训练的默认推理流程
│   └── training_process.rs  # 真实主服务训练上传、音色保存重启、版本试听启用、取消及 SIGTERM 子树清理测试
├── Cargo.toml  # 该 Rust 包的名称、workspace 配置与模块依赖声明
└── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
```

可继续查看各子目录的索引：

- [src/](src/DIRECTORY.md)：主服务启动、配置解析及 HTTP / WebSocket 适配源码
- [tests/](tests/DIRECTORY.md)：主服务配置、HTTP、桥接与完整播报集成测试

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 357a1557faed33c62265413abd84122b2f0b7185170d658c236fd5a1e5e0af6d -->
