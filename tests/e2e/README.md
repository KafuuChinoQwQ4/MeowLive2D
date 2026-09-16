# 端到端验证

M6 的统一部署顺序、固定输入、故障演练、5 分钟演示、10 分钟只读观测命令和 M0-M6 结果矩阵见 `docs/acceptance.md`。观测工具测试入口为：

```bash
node --test scripts/acceptance.test.mjs
node scripts/acceptance.mjs --help
```

`scripts/acceptance.mjs` 默认只以 GET 查询主服务，不提交事件、不暂停或恢复 Agent、不控制 OBS，也不自动认定验收通过。其 P50/P95 是 HTTP 状态查询耗时；实际发声、VTS 口型和 OBS 声画必须由 Windows 实机证据确认。

自动化工程链路见 `apps/server/tests/runtime_loop.rs`；模拟后端不产生声音。前端组件测试不代替设备联调。

VTS 协议测试使用受控 WebSocket 服务，见 `crates/desktop-runtime/tests/avatar_*.rs`；`avatar_cli.rs` 启动真实客户端程序核对配置加载、模拟播放口型和断线归零，`avatar_stop.rs` 检查 VTS 不回应时主服务停止回执和口型归零仍及时完成。能量、平滑与生命周期分别在 `output_meter.rs`、`simulated_levels.rs` 和 `lip_sync_*.rs` 测试。

M2 自动化联调见 `apps/server/tests/agent_process.rs`：启动真实主服务可执行程序，通过环境变量传入测试密钥，连接受控 HTTP LLM/TTS 与静音客户端，直到事件状态为 completed。`agent_runtime.rs` 验证回执与断线，`agent_cancellation.rs` 验证模型等待取消；`agent_receipt_retention.rs` 防止语音历史裁剪覆盖 Agent 已完成结果。这些测试未调用真实付费模型。

Windows 实机验收依次执行：

1. 启动配置好参考素材的 GPT-SoVITS 和主服务，启动 Windows `meowlive-client`（不带 `--simulate`）。
2. 面板提交固定文字，确认 audible 音频与 started/completed 回执一致。
3. 切换或关闭页面，确认播放生命周期不受页面影响。
4. 播放期间停止，确认设备静音、旧片丢弃、随后新任务可播。
5. 断网、重启服务、拔插设备，确认 unknown/failed 状态、停止及重新配对，不重复播报。
6. 开启 VTS 插件 API，在客户端本地配置启用 VTS，完成首次插件授权，将 `MeowMouthOpen` 映射至模型嘴部开合参数。确认重启客户端复用授权、拒绝授权不连续弹窗。
7. 播放含静音段的固定语音，确认设备实际发声时嘴部变化、静音时闭嘴。核对立即停止、设备故障和服务断线后的复位；VTS 关闭期间仍能播音和停止，恢复连接先闭嘴、不重放旧口型。
8. 在 OBS 手动配置画面及客户端音频采集，显式开始录制并测量语音/口型偏差和资源指标；OBS 初始化和重连不得自动录制或推流。

本轮尚未执行以上 Windows/GPU/VTS/OBS 验收，也不生成安装包，不能将已有跨平台测试标记为 M1 或 M6 实机验收完成。

M2 真实提供商验收：

1. 配置可用 LLM API 根地址、模型与主服务密钥环境变量，启动后确认默认暂停。
2. 桌面执行端连接后，粘贴 `tests/fixtures/agent-events.json`，确认首次接收 3 条、忽略 1 条重复，再次提交均为重复。
3. 恢复 Agent，核对候选事件、实际朗读文本及礼物总数；不能把 queued 或 ready 当成已感谢。检查原始礼物都关联同一播报，只有听到播放结束后变为 completed。
4. 在模型生成期间暂停、修改人设及立即停止，确认旧回复不继续入队；暂停已在播放的发言时允许当前语音完成，立即停止则须静音。
5. 开启主动发言，确认队列忙碌时不触发、恢复后的首次发言与后续发言均遵守冷却；关闭开关后无事件时不调用模型。
6. 断开桌面连接，确认 Agent 暂停，已发送但未确认播放的结果为 unknown；重连不重播，需人工恢复。服务重启重新加载 TOML，内存事件和面板设置不会被恢复。

真实模型的结构化输出稳定性、回答质量、费用、延迟以及真实平台事件接入尚待相应环境验证。


M3 真实直播平台验收（需要授权账号与房间）：

1. 在主服务环境配置官方访问密钥和主播身份码；设置 `[live].enabled=true` 与合法 `app_id`，启动后确认不会自动连接。
2. 在面板连接直播间，核对房间号、授权结果和连接状态；凭据缺失/过期应明确失败，不出现在响应、面板或日志。
3. 连接 Windows 执行端并恢复 Agent，发送真实弹幕和连续同种礼物；核对事件进入、礼物分组、语音和 completed 回执。
4. 核对重复消息 ID 在去重窗口内不重复感谢；超过内存窗口或服务重启不保证去重。
5. 中断平台网络，确认面板重连且 Agent 暂停；已下发语音继续回执。网络恢复后手动恢复 Agent，不自动重播未知发言。
6. 在连接中、重连等待中和正常收事件时点击断开，确认不再入队且平台项目结束；退出主服务同样检查会话清理。
7. 连续运行至少一个完整心跳周期并观测两类心跳；录制实际 Windows 音频、VTS 口型与 OBS 画面。

本轮未执行真实平台和设备验收。受控 HTTP/WebSocket、测试模型和静音运行时只能证明工程接线与错误路径。


M3 受控验证入口：`apps/server/tests/live_runtime.rs` 串联官方直播源、LLM、GPT-SoVITS、主服务与静音设备，验证重复礼物只有一次 completed；`live_process.rs` 验证真实可执行入口的环境配置和不自动连接。`live_cleanup.rs` 与 `crates/adapters/tests/bilibili_resilience.rs` 覆盖会话清理、心跳并发、同包事件和取消回归。
