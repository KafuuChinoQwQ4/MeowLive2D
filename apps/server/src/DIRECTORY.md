# src 目录索引

主服务启动、配置解析及 HTTP / WebSocket 适配源码

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
src/  # 主服务启动、配置解析及 HTTP / WebSocket 适配源码
├── agent/  # Agent 服务状态、跨端映射和异步模型调度
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── mapping.rs  # 统一事件及 Agent 业务状态到公开 HTTP DTO 的映射
│   ├── state.rs  # Agent 查询控制、原子事件接收及语音状态同步
│   └── worker.rs  # 锁外模型调用、有界重试、GPU 在途互斥及取消核对后的语音入队
├── config/  # 按 Agent 与模型能力拆分的配置校验
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── agent.rs  # Agent 人设和有界调度参数的 TOML 配置
│   ├── live.rs  # 官方直播接入开关、应用编号、凭据变量名和重连参数校验
│   ├── llm.rs  # LLM 提供商协议、地址、模型及资源限额校验与密钥脱敏
│   ├── resources.rs  # Linux 资源保存目录和引擎共享挂载配置
│   └── training.rs  # 训练路径、本地识别模型、超时及受管推理默认权重配置校验
├── live/  # 官方直播源组装及异步连接、接收、清理与重连驱动
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── bootstrap.rs  # 从主服务环境变量组装官方直播适配器
│   └── worker.rs  # 直播事件接收、去重入队、取消、会话清理和退避重连
├── transport/  # 控制接口、跨端连接与协议到领域对象的转换
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── agent.rs  # Agent 状态控制与批量事件输入的 HTTP 边界
│   ├── bridge.rs  # 独立控制与音频发送循环、心跳及执行回执接收
│   ├── error.rs  # 稳定的结构化 HTTP 错误映射
│   ├── http.rs  # 组装播报、Agent、事件、直播连接及 WebSocket 路由和来源校验
│   ├── live.rs  # 直播连接查询、连接及断开的 HTTP 输入边界
│   ├── llm.rs  # LLM 接入配置读写与草稿连接测试 HTTP 入口
│   ├── mapping.rs  # protocol DTO 与 domain 类型的显式转换，避免序列化字段影响领域规则。
│   ├── mod.rs  # HTTP / WebSocket 输入与输出适配；在协议 DTO 与领域对象之间进行映射。
│   ├── obs.rs  # 通过桌面资源通道执行 OBS 状态查询与受限控制
│   ├── origin.rs  # 浏览器请求来源校验及 HTTP 来源中间件
│   ├── resources.rs  # 音色和角色增删选择、模型删除代理及能力验证路由
│   ├── runtime.rs  # 离线推理测量、GPU 采样错误传播与预设查询
│   ├── training.rs  # 训练素材导入、性能设置、按所选 GPU 准入及版本试听保存接口
│   ├── training_models.rs  # 模型内存状态查询及带资源互斥和取消保护的启停接口
│   └── websocket.rs  # 唯一执行端连接准入及音频配对校验
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── agent.rs  # Agent 异步驱动与服务状态组装入口
├── bootstrap.rs  # 读取私有配置，组装语音、LLM 和直播适配器并管理服务生命周期
├── config.rs  # TOML 配置、本机 LLM 覆盖加载及启动前全局校验
├── gpu.rs  # 训练试听测量独占租约、保留直播及 Agent 状态的音色切换准入与互斥测试
├── lib.rs  # 可注入适配器的服务模块导出与集成测试入口
├── live.rs  # 直播连接单会话所有权、公开快照和手动连接断开控制
├── llm_settings.rs  # 本机 LLM 覆盖配置的原子保存、凭据隔离与重启状态查询
├── main.rs  # Linux / WSL 主服务入口。业务编排位于 meowlive-application。
├── resources.rs  # 桌面资源请求关联、单操作准入与取消生命周期
├── state.rs  # 语音、Agent、唯一执行桥接和直播连接的共享状态及取消生命周期
└── worker.rs  # 语音任务 I/O 驱动、PCM 分片传输与设备回执等待
```

可继续查看各子目录的索引：

- [agent/](agent/DIRECTORY.md)：Agent 服务状态、跨端映射和异步模型调度
- [config/](config/DIRECTORY.md)：按 Agent 与模型能力拆分的配置校验
- [live/](live/DIRECTORY.md)：官方直播源组装及异步连接、接收、清理与重连驱动
- [transport/](transport/DIRECTORY.md)：控制接口、跨端连接与协议到领域对象的转换

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 5f0b6f5650a277f56f9f5abd1b9965de65c1e96a547b4eeba550ec40126e1ec6 -->
