# 配置边界

示例已由各入口实际解析，未知字段会报错。复制为 `*.local.toml` 或放入 `local/` 保存机器配置；这些路径被忽略。

- `server.example.toml`：监听地址、允许的面板来源、语音队列、GPT-SoVITS、Agent 调度、LLM 请求、直播平台接入、资源和训练。空参考素材允许启动，合成时明确失败；LLM 地址与模型都留空则 Agent 不可恢复。
- `offline.example.toml`：本地 LLM 与 TTS 配置模板，必须填写已安装服务的地址与模型名；面板实际测量通过后才显示本次硬件验证结果。
- `desktop.example.toml`：独立执行客户端的服务地址、缓冲上限、握手和重连时间，以及 VTube Studio 插件连接与口型参数；Windows 使用默认音频设备。
- `VITE_MEOWLIVE_SERVER_URL`：前端服务地址，默认 `http://127.0.0.1:19600`；禁止在前端构建变量放密钥。
- `launcher.example.json`：`./launchers/start.sh` 首次复制到私有 `config/local/launcher.json`，记录主服务 TOML、TTS Python、引擎目录、cuda/cpu 设备及本地密钥文件路径。相对路径均以项目根为基准，也可用 `~/` 表示当前用户主目录；模板使用 `data/environments/gpt-sovits/bin/python` 与 `data/engines/GPT-SoVITS`，需自行安装或修改为实际位置。JSON 不接受额外字段或任意命令。修改后重启启动器生效。

主服务默认只监听本机。跨 Windows/WSL 联调需调整监听地址和客户端 server_url，并使用受信网络。当前没有原生连接令牌认证。

通过 `./launchers/start.sh` 打开网页后，GPT-SoVITS、主服务和 Windows 执行端使用“启动与运行”页开关启停。启动器从主服务 TOML 读取监听与 TTS 端口，两者须为回环地址且不得占用面板的 1420；自定义远程服务继续使用手动开发方式。就绪探测、停止和进程组回收由启动器负责，外部终端启动的服务只显示状态。启动器退出不会关闭外部服务。

异常退出后的显式清理：在 Linux / WSL 的项目根目录运行 `npm run stop`。该命令会核对进程身份并停止当前检出的控制面板、主服务（包括已失去原终端的实例）和 `data/control-panel-inference/` 内的受管 TTS；也会请求本项目受管 Windows 执行端退出。正常退出等待超时后才强制回收，不按端口或通用程序名批量结束进程。完成后可重新运行 `npm start`。

本机密钥文件由 `llmKeyFile` 指定，默认使用私有 `config/local/llm-api-key.txt`（含一行 `sk-…`）；如已在启动器环境设置 `api_key_env` 对应变量则优先使用。文件仅在 Linux 读取，密钥只注入主服务，不返回网页。任意格式的 API 密钥也可从“LLM 接入”页填写，或通过环境变量提供。日志位于 `logs/control-panel/`，每次启动覆盖单服务日志并限制写入量。公共配置只保存路径和环境变量名。

TTS 默认使用 `ttsMemoryMode="low"`：中文采用引擎自带的轻量拼音分支，不加载 G2PW 多音字模型，以降低系统内存占用；少数多音字的准确性可能下降。内存充足且需要完整中文注音时，可在私有 launcher JSON 设为 `"standard"` 后重启面板。直接运行推理脚本对应 `--memory-mode low|standard`。两种模式均保留 CUDA 半精度和单实例推理，外部引擎文件不修改。

启动器会检查 Linux/WSL 和 Windows 主机的可用内存（以及 Windows 可提交余量）；不足 1024 MiB（1 GiB）时拒绝启动 TTS，运行中任一余量低于 768 MiB 时停止由本启动器管理的 TTS，不自动重启。检查约每 5 秒一次，不接管外部服务，也不修改系统内存或交换文件设置；它无法保证避免所有显存不足、驱动故障或黑屏。16 GB 机器建议先使用云端 LLM、单个 TTS 与 VTS，避免同时加载本地 LLM 或训练。

参考录音中的人声、参考文本和所选语言必须逐字对应，不能填写占位数字；上传后即可直接试听，无须先训练。上传音色的目标播报文本单独自动识别语言，不沿用参考录音的语言。TTS 返回完全静音的 WAV 时会显示失败，避免静音任务被误报为播放完成。

声音训练默认只需提供语音素材，由后台自动提取文本后微调；也可选择输入并校对文本，手动填写或提取空白片段的文本，确认后训练。自动提取使用训练 Python 环境中的 `faster-whisper`，在 CPU 上以 int8 运行。`[training].asr_model` 可设置本地模型目录的绝对路径；留空使用 `engine_root/tools/asr/models/faster-whisper-large-v3`。模型目录须包含 `model.bin`、`config.json`、`tokenizer.json`、`preprocessor_config.json`，运行时不会下载。模型语言能力需覆盖所选片段语言；缺少依赖、模型或识别结果为空时会失败，不能用空文本继续训练。手动填写完整文本的流程不依赖识别模型。

参考音频必须由引擎所在 Linux 可访问。M5 的 `[training]` 默认关闭；启用时 `python`、`engine_root` 须为绝对路径，`directory` 相对于配置文件，`timeout_seconds` 为 60–86400 秒。任务在非直播、所选 GPU 显存释放且本地 LLM 停止后运行；受管 TTS 明确报告模型已卸载时可保留其服务端口，否则仍须停止 TTS，项目不停止其他软件的进程。训练输出和缓存只写自有目录，外部引擎安装只读。

试听与版本启用要求 `managed_inference=true`、默认 v2 GPT/SoVITS 两个权重的绝对路径，并使用 `scripts/start-managed-inference.py` 创建的独立推理实例；该实例使用私有可写配置。面板的 TTS 开关已使用此脚本，手动启动方法见脚本的 `--help`。每次合成按音色切换成对权重。服务启动时仅提供 API，模型默认为关闭；在“训练与离线”点击“启用语音模型”后才加载权重。关闭模型会释放推理模型及相关缓存，保留权重文件和已选音色；再次启用后，下次合成仍按已选音色加载权重。此开关同时控制默认模型和训练模型的推理驻留，不控制训练任务的暂停续训。更新后需要重启主服务与受管 TTS 才能使用新接口；旧实例会显示不支持独立开关。未启用训练时保留既有参考音色推理能力；训练素材要求见 [新手说明](../README.md#想进一步训练声音)。2026-09-17 本机 RTX 4050 Laptop（6141 MiB 显存）通过真实训练接口完成 SoVITS 与 GPT 各 5 轮，约 11 分 10 秒生成成对权重并通过校验，任务达到 100%。验收使用重复的参考音频与已校对文本，仅覆盖训练流程，未验证该模型的试听效果；本机缺少 ASR 模型，仅语音模式仍需先安装本地识别模型。

训练性能参数随每次任务保存：`batch_size` 为 1–16，`data_workers` 为 0–8，`cpu_threads` 为 1–16，`gpu_index` 为 0–15，`low_memory` 为布尔值。默认依次为 1、1、2、0、true；未包含参数的旧任务使用这些默认值。页签中提供省内存（1/1/2）、均衡（2/2/4）、高吞吐（4/4/8）和自定义设置。GPU 编号选择一张物理显卡，并非多卡并行；当前训练仍要求 CUDA 和 FP16。小数据集的 GPT 阶段保留上游批大小保护，实际批大小可能小于所选值。训练前检查选中 GPU 的占用不超过 512 MiB；参数选项未做多种硬件实测。

仅使用参考音色时无需开启训练或配置权重管理，也可以通过受管 TTS 的专用接口启停默认模型。模型状态接口为 `GET /api/training/models`，控制为 `POST /api/training/models`，请求 `{"enabled":true}` 或 `{"enabled":false}`。加载、卸载、合成和训练有互斥保护；页面断开不会提前释放正在执行的操作。卸载模型后进程及 CUDA 上下文仍可能占用基础内存，停止 TTS 服务才会退出整个进程。


示例中 VTS 默认关闭；面板启动 Windows 执行端前会将实际使用的 `target/windows-client/desktop.local.toml` 中 `[vtube_studio].enabled` 写为 `true`，无须修改示例文件。直接启动客户端时仍需手动启用。启用后，客户端独立连接 `websocket_url`，首次由 VTS 弹窗授权。`token_path` 相对于 TOML 所在目录解析，默认保存到 `config/local/vts-token.json`；授权文件不进入前端或日志。Unix 文件权限须为 `0600`，Windows 应保存在当前用户的私有目录。拒绝、撤销授权或授权超时后不会反复弹窗；确认 VTS 设置、移走失效的本地授权文件并重启客户端后重新授权。已有授权文件不会自动覆盖。

客户端创建范围 `0..1` 的 `MeowMouthOpen` 自定义输入；在 VTS 的模型参数配置中手动映射到该模型的嘴部开合参数。可通过 `mouth_parameter` 改名（4–32 位 ASCII 字母或数字）。嘴部映射未配置时，即使客户端显示 `Connected`，模型也不会自动张嘴。插件授权与连接状态目前输出在桌面进程日志中。

`[lip_sync]` 使用设备输出样本的 RMS：`noise_floor` 为静音阈值（0–0.5），`gain` 为放大系数（0.1–20），`attack_ms`/`release_ms` 为开口/闭口平滑时间（1–1000 / 1–2000 毫秒），`update_hz` 为观测频率（10–30）。VTS 常规注入最多 30 次/秒，仅保留最新值；停止立即发布零值，超过 250 毫秒未更新的值按静音处理。设备延迟前、任务完成、失败和连接断开时均复位。VTS 故障独立于播放连接，不影响停止回执；目标应用失联时无法保证立即改变画面，重连会先发零值。

Agent 启动始终暂停。`[agent]` 的 `persona` 为 1–2000 个 Unicode 字符，`topic` 可空、最多 200 字；`cooldown_ms` 为 1000–3600000 毫秒，成功、失败、忽略及播放终态都会进入冷却。只有 `proactive_enabled=true` 且无事件、无播报时才会主动发言，首次恢复后也要等待冷却。

调度上限为 `pending_capacity` 1–512、终态 `history_limit` 1–2000、`dedup_capacity` 1–4096、`batch_size` 1–16、已播完 `conversation_limit` 1–20 轮；`event_ttl_ms` 为 1000–600000，`gift_merge_ms` 为 0–10000（0 禁用分组）。去重 ID 在会话内全局唯一，平台适配器须给原始平台 ID 加来源前缀；活动 ID 不因去重缓存淘汰而失效，终态 ID 超出缓存后可被重新接收。去重不提供跨重启保障。

`[llm]` 的 `base_url`/`model` 同时留空可保留人工播报模式；填写后会在启动时校验并读取 `api_key_env` 指定的环境变量。环境变量缺失或空值时 Agent 保持未配置，主服务继续运行以便从面板修复；`api_key_env=""` 显式选择无认证模式。此适配器直连配置地址，不自动继承 HTTP 代理、不跟随重定向，错误不公开响应正文或密钥。模型名最多 128 字节，`max_tokens` 为 64–4096，`max_response_bytes` 为 1024–1048576。`timeout_seconds` 为 1–120 秒，覆盖一轮全部尝试及重试等待；`max_retries` 为 0 或 1，只重试超时、连接失败、429 和 5xx 等临时错误。

“LLM 接入”页支持 GPT、Claude、Kimi、GLM、Grok、Gemini、DeepSeek 预设与自定义地址、模型、密钥。`api_format` 可设 `openai_chat`（默认）、`openai_responses`、`anthropic_messages`、`gemini_generate_content`；按协议构造认证头、请求及响应。地址可填含 API 版本路径的根地址或完整端点，不能含查询参数、凭据或片段。JSON 模式不受支持时可关闭，Claude 通过提示约束 JSON；所有格式仍校验 Agent 决策。

`GET /api/llm/settings` 读取待启用设置与密钥存在标记，`POST /api/llm/settings` 保存，`POST /api/llm/test` 对当前草稿执行一次模型测试。保存后重启主服务生效，页面会提示待重启状态。测试不保存且不播放声音。面板保存位置为 TOML 同级 `local/<文件名去掉扩展名>-llm.json`，例如 `config/local/server.local-llm.json`，优先于 `[llm]`；密钥以本机明文保存，Unix 文件权限 `0600`，查询及日志不公开密钥。移走该文件并重启可恢复 TOML 配置。密钥留空仅保留相同服务商、格式和地址的密钥；更换目标必须重新填写或明确清除。

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
