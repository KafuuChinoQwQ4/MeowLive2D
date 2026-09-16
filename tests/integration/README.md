# 集成验证

执行 `cargo test -p meowlive-adapters -p meowlive-server -p meowlive-desktop-runtime`。测试注册在相应 crate 的 tests 目录，按 HTTP/WAV、连接配对、执行回执、停止取消、runtime 联调等职责拆分。

测试用真实本地 TCP/HTTP/WebSocket 与受控 PCM 夹具，不连接真实 GPT-SoVITS。`apps/server/tests/runtime_loop.rs` 将真实服务与桌面连接循环组合，使用显式静音后端确认完成回执。真实 GPU 延迟、音质、显存与模型权重需单独配置和记录。
