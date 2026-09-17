# transport 目录索引

控制接口、跨端连接与协议到领域对象的转换

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
transport/  # 控制接口、跨端连接与协议到领域对象的转换
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── agent.rs  # Agent 状态控制与批量事件输入的 HTTP 边界
├── bridge.rs  # 独立控制与音频发送循环、心跳及执行回执接收
├── error.rs  # 稳定的结构化 HTTP 错误映射
├── http.rs  # 组装播报、Agent、事件、直播连接及 WebSocket 路由和来源校验
├── live.rs  # 直播连接查询、连接及断开的 HTTP 输入边界
├── llm.rs  # LLM 接入配置读写与草稿连接测试 HTTP 入口
├── mapping.rs  # protocol DTO 与 domain 类型的显式转换，避免序列化字段影响领域规则。
├── mod.rs  # HTTP / WebSocket 输入与输出适配；在协议 DTO 与领域对象之间进行映射。
├── obs.rs  # 通过桌面资源通道执行 OBS 状态查询与受限控制
├── origin.rs  # 浏览器请求来源校验及 HTTP 来源中间件
├── resources.rs  # 音色和角色增删选择、模型删除代理及能力验证路由
├── runtime.rs  # 离线推理测量、GPU 采样错误传播与预设查询
├── training.rs  # 训练素材导入、性能设置、按所选 GPU 准入及版本试听保存接口
├── training_models.rs  # 模型内存状态查询及带资源互斥和取消保护的启停接口
└── websocket.rs  # 唯一执行端连接准入及音频配对校验
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 637ac8b3a9e82a14720d48aab02616170feb54e2ce8cc2d287f4526766a1748f -->
