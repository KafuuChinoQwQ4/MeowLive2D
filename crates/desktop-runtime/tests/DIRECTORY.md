# tests 目录索引

桌面执行运行时独立集成测试

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
tests/  # 桌面执行运行时独立集成测试
├── avatar_support/  # VTS 本地 WebSocket 场景测试的隔离文件与协议辅助设施
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── mod.rs  # 受控 VTS 服务器、授权交互与参数断言共用辅助
├── connection/  # 使用虚拟时间和真实 WebSocket 的连接期限测试
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── flush.rs  # 阻塞 Pong 写回的超时测试
│   └── liveness.rs  # 双通道独立心跳期限、续期及设备清理测试
├── lip_sync_support/  # 口型观察测试的共享可控设备夹具
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── mod.rs  # 可设置能量、事件与操作错误的测试音频设备
├── support/  # 运行时测试共用手动设备与消息夹具
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── mod.rs  # 可控设备事件及播报消息测试夹具
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── avatar_authorization.rs  # VTS 首次授权、令牌复用、拒绝撤销、授权中退出与断线测试
├── avatar_cli.rs  # 真实客户端进程的 VTS 连接、配置加载与断线归零联调
├── avatar_config_validation.rs  # VTS 默认关闭、未知键与配置边界校验测试
├── avatar_configuration.rs  # 客户端 VTS 和口型配置的兼容、校验与相对路径测试
├── avatar_lifecycle.rs  # VTS 表现输出组装在禁用与等待授权时的关闭测试
├── avatar_parameters.rs  # VTS 最新值映射、参数范围、注入心跳、限频和过期归零测试
├── avatar_protocol.rs  # VTS 响应类型与请求关联校验、总超时和错误信息脱敏测试
├── avatar_reconnect.rs  # VTS 重连先归零、服务不可用和未响应请求中的退出测试
├── avatar_resources.rs  # VTS 模型与热键请求和错配边界测试
├── avatar_stop.rs  # VTS 请求未回复时主服务停止回执与口型归零的隔离测试
├── avatar_switching.rs  # 角色口型切换归零、应用确认与禁用驱动测试
├── avatar_token_storage.rs  # VTS 本地令牌长度、字符、文件权限、符号链接和磁盘等待取消测试
├── client_cli.rs  # 客户端帮助与缺失配置退出行为测试
├── client_configuration.rs  # TOML、配对 URL 与平台设备边界测试
├── device_conversion.rs  # PCM 声道映射与跨分片重采样测试
├── device_timing.rs  # 设备延迟、播放开始与完成时钟测试
├── host_lifecycle.rs  # 宿主启动失败、离线取消和双通道退出清理测试
├── lip_sync_delivery.rs  # 口型发布频率、静音保鲜、最新值覆盖与停止抢占测试
├── lip_sync_failures.rs  # 设备启动、写入和结束失败时的停止与口型复位测试
├── lip_sync_levels.rs  # 口型能量阈值、增益、时间平滑及非法输入测试
├── lip_sync_lifecycle.rs  # 设备驱动口型、停止、完成、失败和析构复位测试
├── model_assets.rs  # 模型引用、路径限制、安装与覆盖保护测试
├── obs_configuration.rs  # OBS 本机配置、URL 归一化及超时边界测试
├── obs_websocket.rs  # OBS 受控鉴权、状态读回、协议边界与禁止重放写请求测试
├── output_meter.rs  # 设备能量的播放延迟、静音、过期、容量与复位测试
├── playback_completion.rs  # 播放完成与设备失败回执测试
├── playback_ordering.rs  # 控制音频竞态、乱序及有界待播缓存测试
├── playback_stop.rs  # 停止代次、迟到音频及断线清理测试
├── simulated_levels.rs  # 模拟播放电平的时间位置、停止、完成与缓冲上限测试
├── websocket_disconnect.rs  # 音频连接中断时停止设备与关闭控制连接测试
└── websocket_session.rs  # 双 WebSocket 配对与实际客户端回执闭环测试
```

可继续查看各子目录的索引：

- [avatar_support/](avatar_support/DIRECTORY.md)：VTS 本地 WebSocket 场景测试的隔离文件与协议辅助设施
- [connection/](connection/DIRECTORY.md)：使用虚拟时间和真实 WebSocket 的连接期限测试
- [lip_sync_support/](lip_sync_support/DIRECTORY.md)：口型观察测试的共享可控设备夹具
- [support/](support/DIRECTORY.md)：运行时测试共用手动设备与消息夹具

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 5fe9c65ed7eb38e8a1cc07986db4632603867800a6ec1f544da228156a9cfa8f -->
