# 配置边界

示例已由各入口实际解析，未知字段会报错。复制为 `*.local.toml` 或放入 `local/` 保存机器配置；这些路径被忽略。

- `server.example.toml`：监听地址、允许的面板来源、语音队列、GPT-SoVITS、Agent 调度、LLM 请求、直播平台接入、资源和训练。空参考素材允许启动，合成时明确失败；LLM 地址与模型都留空则 Agent 不可恢复。
- `offline.example.toml`：本地 LLM 与 TTS 配置模板，必须填写已安装服务的地址与模型名；面板实际测量通过后才显示本次硬件验证结果。
- `desktop.example.toml`：独立执行客户端的服务地址、缓冲上限、握手和重连时间，以及 VTube Studio 插件连接与口型参数；Windows 使用默认音频设备。
- `VITE_MEOWLIVE_SERVER_URL`：前端服务地址，默认 `http://127.0.0.1:19600`；禁止在前端构建变量放密钥。
- `launcher.example.json`：`./launchers/start.sh` 首次复制到私有 `config/local/launcher.json`，记录主服务 TOML、TTS Python、引擎目录、cuda/cpu 设备及本地密钥文件路径。相对路径均以项目根为基准，也可用 `~/` 表示当前用户主目录；模板使用 `data/environments/gpt-sovits/bin/python` 与 `data/engines/GPT-SoVITS`，需自行安装或修改为实际位置。JSON 不接受额外字段或任意命令。修改后重启启动器生效。

主服务默认只监听本机。跨 Windows/WSL 联调需调整监听地址和客户端 server_url，并使用受信网络。当前没有原生连接令牌认证。

通过 `./launchers/start.sh` 打开网页后，GPT-SoVITS、主服务和 Windows 执行端使用“启动与运行”页开关启停。启动器从主服务 TOML 读取监听与 TTS 端口，两者须为回环地址且不得占用面板的 1420；自定义远程服务继续使用手动开发方式。就绪探测、停止和进程组回收由启动器负责，外部终端启动的服务只显示状态。启动器退出不会关闭外部服务。

本机密钥文件由 `llmKeyFile` 指定，默认使用私有 `config/local/llm-api-key.txt`（含一行 `sk-…`）；如已在启动器环境设置 `api_key_env` 对应变量则优先使用。文件仅在 Linux 读取，密钥只注入主服务，不返回网页。其他格式的 API 密钥可通过环境变量提供。日志位于 `logs/control-panel/`，每次启动覆盖单服务日志并限制写入量。公共配置只保存路径和环境变量名。

参考音频必须由引擎所在 Linux 可访问。M5 的 `[training]` 默认关闭；启用时 `python`、`engine_root` 须为绝对路径，`directory` 相对于配置文件，`timeout_seconds` 为 60–86400 秒。任务在非直播且推理端口及显存释放后运行，项目不停止其他软件的进程。训练输出和缓存只写自有目录，外部引擎安装只读。

试听与版本启用要求 `managed_inference=true`、默认 v2 GPT/SoVITS 两个权重的绝对路径，并使用 `scripts/start-managed-inference.py` 创建的独立推理实例；该实例使用私有可写配置。每次合成按音色切换成对权重。未启用训练时保留既有参考音色推理能力。启动命令、素材限制及硬件验收边界见根 README 的 M5 说明。

VTS 默认关闭；启用 `[vtube_studio].enabled` 后，客户端独立连接 `websocket_url`，首次由 VTS 弹窗授权。`token_path` 相对于 TOML 所在目录解析，默认保存到 `config/local/vts-token.json`；授权文件不进入前端或日志。Unix 文件权限须为 `0600`，Windows 应保存在当前用户的私有目录。拒绝、撤销授权或授权超时后不会反复弹窗；确认 VTS 设置、移走失效的本地授权文件并重启客户端后重新授权。已有授权文件不会自动覆盖。

客户端创建范围 `0..1` 的 `MeowMouthOpen` 自定义输入；在 VTS 的模型参数配置中手动映射到该模型的嘴部开合参数。可通过 `mouth_parameter` 改名（4–32 位 ASCII 字母或数字）。嘴部映射未配置时，即使客户端显示 `Connected`，模型也不会自动张嘴。插件授权与连接状态目前输出在桌面进程日志中。

`[lip_sync]` 使用设备输出样本的 RMS：`noise_floor` 为静音阈值（0–0.5），`gain` 为放大系数（0.1–20），`attack_ms`/`release_ms` 为开口/闭口平滑时间（1–1000 / 1–2000 毫秒），`update_hz` 为观测频率（10–30）。VTS 常规注入最多 30 次/秒，仅保留最新值；停止立即发布零值，超过 250 毫秒未更新的值按静音处理。设备延迟前、任务完成、失败和连接断开时均复位。VTS 故障独立于播放连接，不影响停止回执；目标应用失联时无法保证立即改变画面，重连会先发零值。

Agent 启动始终暂停。`[agent]` 的 `persona` 为 1–2000 个 Unicode 字符，`topic` 可空、最多 200 字；`cooldown_ms` 为 1000–3600000 毫秒，成功、失败、忽略及播放终态都会进入冷却。只有 `proactive_enabled=true` 且无事件、无播报时才会主动发言，首次恢复后也要等待冷却。

调度上限为 `pending_capacity` 1–512、终态 `history_limit` 1–2000、`dedup_capacity` 1–4096、`batch_size` 1–16、已播完 `conversation_limit` 1–20 轮；`event_ttl_ms` 为 1000–600000，`gift_merge_ms` 为 0–10000（0 禁用分组）。去重 ID 在会话内全局唯一，平台适配器须给原始平台 ID 加来源前缀；活动 ID 不因去重缓存淘汰而失效，终态 ID 超出缓存后可被重新接收。去重不提供跨重启保障。

`[llm]` 的 `base_url`/`model` 同时留空可保留人工播报模式；填写后会在启动时校验并读取 `api_key_env` 指定的环境变量。环境变量缺失或空值会报错；`api_key_env=""` 显式选择无认证模式。此适配器直连配置地址，不自动继承 HTTP 代理、不跟随重定向，错误不公开响应正文或密钥。模型名最多 128 字节，`max_tokens` 为 64–4096，`max_response_bytes` 为 1024–1048576。`timeout_seconds` 为 1–120 秒，覆盖一轮全部尝试及重试等待；`max_retries` 为 0 或 1，只重试超时、连接失败、429 和 5xx 等临时错误。

`mode` 默认 `cloud`；设为 `local` 时 LLM/TTS 地址必须使用回环 IP，`api_key_env` 必须为空，`max_tokens` 不超过 1024，`max_retries=0`。这些约束不等于完整离线验收；面板测量真实 LLM→TTS 的耗时及显存，并在重启、训练或启用新版本后清除旧验证标记。

首版以完整非流式 JSON 决策生成一条语音：`reply_to` 是本轮候选 ID 子集，`text` 为 1–500 字或 null，`topic` 为最多 200 字或 null。多余动作字段、工具调用、截断、未知 ID 和礼物组部分选择会失败，不执行任意控制命令。输入事件与历史作为 user 数据，人设才进入 system 指令。兼容协议不等于已验证每家提供商；首次真实调用前应使用小批事件核对响应与模型计费。

Agent HTTP 接口为 `GET /api/agent`，`POST /api/agent/settings`、`/api/agent/pause`、`/api/agent/resume`，以及 `POST /api/events`（`{"events":[...]}`，每批 1–100 条，256 KiB 上限，整体校验后接收）。这些入口沿用主服务来源校验。面板修改保留在内存，重启后回到 TOML 配置；原 `/api/stop` 会同时暂停 Agent。


`[live]` 是哔哩哔哩官方直播开放平台配置。`enabled` 默认 false，启用时 `app_id` 须为 1–9223372036854775807；访问密钥 ID、密钥、主播身份码仅由 `access_key_id_env`、`access_key_secret_env`、`identity_code_env` 指定的环境变量读取。缺失或空值时直播接入不可用，其他服务可继续运行。启动不自动授权，面板“连接直播间”才调用项目开始接口；官方授权响应决定房间号。

`reconnect_initial_ms` 为 10–60000 ms，`reconnect_max_ms` 不小于初始值且不超过 300000 ms；默认从 1 秒指数退避至 30 秒。临时网络错误进入重连，永久鉴权或协议错误进入失败；清理项目失败会停止自动重试并展示错误，避免连续创建无法核对的会话。手动断开期间拒绝新连接，直至旧会话清理结束。平台失联暂停 Agent，恢复连接不自动恢复模型调用。所有状态和计数仅在内存保存。

## M6 桌面与 OBS

Windows Tauri 启动方法见 `apps/desktop/src-tauri/README.md`。主服务的 `allowed_origins` 须包含 `http://tauri.localhost` 以及开发面板地址；桌面 `server_url` 填主服务 HTTP 或 WS origin，不填 `/api` 路径。原生启动地址优先于前端构建变量。跨 WSL 访问时同时调整主服务监听地址与桌面连接地址。

执行端私有 TOML：

```toml
[obs]
enabled = true
websocket_url = "ws://127.0.0.1:4455"
password_env = "MEOWLIVE_OBS_PASSWORD"
timeout_ms = 5000
```

启用 OBS 的 WebSocket v5 服务并在启动执行端的终端设置上述环境变量为 OBS 密码。只接受本机 IP，不接受远程主机、凭据、路径、查询或片段。超时范围 100–10000 ms，覆盖连接、鉴权、指令和状态读回。未启用返回“OBS 控制未启用”；连接失败或结果未知先手工刷新。

面板经 `GET /api/obs` 查询、`POST /api/obs` 控制，指令为 `status`、`set_scene`（含 `scene_name`）、`start_recording` 和 `stop_recording`。不支持推流命令，录制和场景修改不自动重试。OBS 配置、密码以及录制输出路径不经 HTTP 返回。

Windows 开关直接读取 `target/windows-client/desktop.local.toml`，无需把程序复制到 Windows 目录。修改这份配置后重新打开第三个开关；不要把旧的 Windows 手动启动目录配置误认为开关正在读取的配置。自定义主服务端口时，将这份配置的 `server_url` 同步为相同地址。
