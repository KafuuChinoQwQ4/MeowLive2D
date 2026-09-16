# tests 目录索引

主服务配置、HTTP、桥接与完整播报集成测试

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
tests/  # 主服务配置、HTTP、桥接与完整播报集成测试
├── agent_support/  # Agent 服务集成测试公共夹具
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── mod.rs  # 可注入模型与语音的服务和后台任务测试组装
├── live_support/  # 可控直播源与事件的服务生命周期测试支持
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── mod.rs  # 可控直播连接、会话队列、事件与状态等待工具
├── process_support/  # 真实主服务进程测试的隔离目录和音频构造辅助
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── mod.rs  # 主服务测试子进程启动清理和受控 WAV 构造
├── support/  # 主服务集成测试公共夹具
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── mod.rs  # 受控合成器、HTTP 请求与双 WebSocket 连接夹具
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── agent_api.rs  # Agent 默认暂停、设置、事件校验、去重与停止接口测试
├── agent_bootstrap.rs  # LLM 启动配置、缺失环境变量及无认证本地模型测试
├── agent_cancellation.rs  # 暂停停止配置修改和断线取消在途 LLM 的集成测试
├── agent_capacity.rs  # 事件批量请求大小和容量拒绝的原子性测试
├── agent_configuration.rs  # Agent 和 LLM 的默认配置、字段边界及解析错误脱敏测试
├── agent_process.rs  # 真实主服务进程、受控 LLM 与 TTS、静音客户端的播报闭环测试
├── agent_receipt_retention.rs  # 语音历史裁剪时保留 Agent 已完成播放结果的回归测试
├── agent_retries.rs  # 模型临时错误分类和有限重试次数的集成测试
├── agent_runtime.rs  # 模拟事件到设备播放完成及断线未知状态的集成测试
├── bridge_handshake.rs  # 协议版本、唯一执行端与双连接配对测试
├── configuration.rs  # 示例配置兼容及无效参数拒绝测试
├── http_api.rs  # 状态、播报输入与停止接口测试
├── live_admission.rs  # 直播事件容量丢弃、无效事件及退出清理测试
├── live_cleanup.rs  # 直播延迟清理、清理失败、房间状态与退避取消回归测试
├── live_configuration.rs  # 直播凭据缺失、环境变量边界与配置验证测试
├── live_http.rs  # 直播默认禁用状态、连接入口和配置范围测试
├── live_lifecycle.rs  # 单直播连接、取消迟到结果、事件去重与重连终止测试
├── live_process.rs  # 真实主服务进程的直播凭据组装、默认不连接与公开响应脱敏测试
├── live_runtime.rs  # 受控平台礼物与重复帧经真实适配器、Agent、语音和静音设备完成回执的联调测试
├── m5_config.rs  # 本地预设地址资源上限及训练配置约束测试
├── obs_http.rs  # OBS HTTP 参数、桌面连接要求及模拟执行端贯通测试
├── request_origin.rs  # HTTP 来源拒绝和无副作用保障测试
├── resources_bridge.rs  # 桌面资源请求关联、停止、断连及响应边界测试
├── resources_characters.rs  # 角色加载确认、预览验证及映射变更失效测试
├── resources_http.rs  # 资源初始状态、当前音色别名与离线操作测试
├── resources_process.rs  # 真实服务进程上传、恢复、试听、Agent 与缺失资源链路测试
├── resources_runtime.rs  # 面板经真实桌面执行库到受控 VTS 的角色、热键与停止链路测试
├── runtime_loop.rs  # 真实服务与独立桌面运行时的静音播放集成测试
├── speech_delivery.rs  # PCM 下发、设备回执门控及独立停止通道测试
├── synthesis_cancellation.rs  # 合成停止、断线未知与重连不重播集成测试
├── training_http.rs  # 训练未配置、音色保存接口和离线未验证的公开 HTTP 行为测试
└── training_process.rs  # 真实主服务训练上传、音色保存重启、版本试听启用、取消及 SIGTERM 子树清理测试
```

可继续查看各子目录的索引：

- [agent_support/](agent_support/DIRECTORY.md)：Agent 服务集成测试公共夹具
- [live_support/](live_support/DIRECTORY.md)：可控直播源与事件的服务生命周期测试支持
- [process_support/](process_support/DIRECTORY.md)：真实主服务进程测试的隔离目录和音频构造辅助
- [support/](support/DIRECTORY.md)：主服务集成测试公共夹具

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: ffb18c18f17eef25477341cc9e615f3a58176d6ee3a0479e71ed43011877a1bb -->
