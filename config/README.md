# 配置边界

## 观众记忆数据库环境

Windows App 用户请先看 [数据库下载与启用](windows-database.md)：包含官方下载入口、配套容器初始化、连接变量、观众存储开关和验证步骤。下文自动准备行为指 Linux / WSL 源码启动器。

`databases.compose.yaml` 为观众记忆功能提供独立数据库环境。PostgreSQL 默认保存观众身份、幂等事件、陪伴积分和记忆；控制面板用户直接拥有管理权限，无需另行登录。独立模型提取和 Neo4j 图投影按需配置。

| 服务 | 本项目地址 | 数据与账号 |
| --- | --- | --- |
| PostgreSQL 16 + pgvector 0.8.6 | `127.0.0.1:25432` | 数据库 `meowlive`；应用账号 `meowlive_app`，默认 schema `app` |
| Neo4j 5.26.30 Community | HTTP `http://localhost:17474`；Bolt `bolt://localhost:17687` | 数据库 `neo4j`；账号 `neo4j`，使用本项目独立密码 |

MemoChat 的 PostgreSQL 使用 `15432`，Neo4j 使用 `7474/7687`。新环境使用独立容器、网络、账号及绑定目录；不会重用旧数据目录。Community 版本只有一个标准业务数据库，因此这里运行独立 Neo4j 实例。

所有命令在项目根目录执行。凭据位于 Git 忽略的 `config/local/databases.env`，格式见 `databases.env.example`；每个密码应单独生成。通过面板或 `npm run start:server` 启动主服务时，缺少该文件会自动生成随机凭据。手动准备时也可执行以下命令，已有文件会报错并保留原值：

```bash
python3 - <<'PY'
from pathlib import Path
import os, secrets
directory = Path('config/local')
directory.mkdir(parents=True, exist_ok=True, mode=0o700)
fd = os.open(directory / 'databases.env', os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
with os.fdopen(fd, 'w') as output:
    for key in ('MEOWLIVE_POSTGRES_ADMIN_PASSWORD', 'MEOWLIVE_POSTGRES_APP_PASSWORD', 'MEOWLIVE_NEO4J_PASSWORD'):
        output.write(key + '=' + secrets.token_hex(32) + '\n')
PY
```

启动、查看状态、停止：

```bash
docker compose --env-file config/local/databases.env -f config/databases.compose.yaml up -d --wait --wait-timeout 180
docker compose --env-file config/local/databases.env -f config/databases.compose.yaml ps
docker compose --env-file config/local/databases.env -f config/databases.compose.yaml stop
```

数据分别位于 `data/databases/postgres`、`data/databases/neo4j`，Neo4j 日志在 `logs/databases/neo4j`；停止服务保留数据。默认开启主服务时会启动并等待本项目 PostgreSQL 就绪；Neo4j 仍需手动启动。关闭主服务或面板会保留数据库运行和数据，不自动随 Docker 重启；需要停止时使用上面的专用命令。显式提供数据库连接变量时使用该数据库，不管理本地容器。

PostgreSQL 内存上限 512 MiB，Neo4j 上限 1 GiB，两者 CPU 上限各为 1 核；这是上限，不是预先占满的内存。两个项目同时运行仍共享 WSL 内存、CPU 和磁盘，语音训练时可停止本项目数据库以释放资源。

`postgres-init.sql` 只在全新数据目录首次初始化时执行：在 `meowlive` 数据库启用 `vector`，建立非超级用户应用账号和 `app` schema。管理账号 `meowlive_admin` 仅用于数据库维护。更改 env 文件不会自动修改已有数据库密码，也不会重新执行初始化 SQL；不要为重设密码删除数据目录。配置展开检查请用 `config --quiet`，避免把含密码的完整配置输出到日志。

## 主服务与执行端配置

示例已由各入口实际解析，未知字段会报错。复制为 `*.local.toml` 或放入 `local/` 保存机器配置；这些路径被忽略。

- `server.example.toml`：监听地址、允许的面板来源、语音队列、GPT-SoVITS、Agent 调度、LLM 请求、直播平台接入、资源和训练。空参考素材允许启动，合成时明确失败；LLM 地址与模型都留空则 Agent 不可恢复。
- `offline.example.toml`：本地 LLM 与 TTS 配置模板，必须填写已安装服务的地址与模型名；面板实际测量通过后才显示本次硬件验证结果。
- `desktop.example.toml`：独立执行客户端的服务地址、缓冲上限、握手和重连时间，以及 VTube Studio 插件连接与口型参数；Windows 使用默认音频设备。
- `VITE_MEOWLIVE_SERVER_URL`：前端服务地址，默认 `http://127.0.0.1:19600`；禁止在前端构建变量放密钥。
- `launcher.example.json`：`./launchers/start.sh` 首次复制到私有 `config/local/launcher.json`，记录主服务 TOML、TTS Python、引擎目录、cuda/cpu 设备及本地密钥文件路径。相对路径均以项目根为基准，也可用 `~/` 表示当前用户主目录；模板使用 `data/environments/gpt-sovits/bin/python` 与 `data/engines/GPT-SoVITS`，需自行安装或修改为实际位置。JSON 不接受额外字段或任意命令。修改后重启启动器生效。

主服务默认只监听本机。跨 Windows/WSL 联调需调整监听地址和客户端 server_url，并使用受信网络。主服务启用 `[auth]` 后，Windows 执行端必须通过私有 `device_token_file` 发送独立设备凭据。

通过 `./launchers/start.sh` 打开网页后，GPT-SoVITS、主服务和 Windows 执行端使用“启动与运行”页开关启停。启动器从主服务 TOML 读取监听与 TTS 端口，两者须为回环地址且不得占用面板的 1420；自定义远程服务继续使用手动开发方式。就绪探测、停止和进程组回收由启动器负责，外部终端启动的服务只显示状态。启动器退出不会关闭外部服务。

异常退出后的显式清理：在 Linux / WSL 的项目根目录运行 `npm run stop`。该命令会核对进程身份并停止当前检出的控制面板、主服务（包括已失去原终端的实例）和 `data/control-panel-inference/` 内的受管 TTS；也会请求本项目受管 Windows 执行端退出。正常退出等待超时后才强制回收，不按端口或通用程序名批量结束进程。完成后可重新运行 `npm start`。

本机密钥文件由 `llmKeyFile` 指定，默认使用私有 `config/local/llm-api-key.txt`（含一行 `sk-…`）；如已在启动器环境设置 `api_key_env` 对应变量则优先使用。文件仅在 Linux 读取，密钥只注入主服务，不返回网页。任意格式的 API 密钥也可从“LLM 接入”页填写，或通过环境变量提供。日志位于 `logs/control-panel/`，每次启动覆盖单服务日志并限制写入量。公共配置只保存路径和环境变量名。

TTS 默认使用 `ttsMemoryMode="low"`：中文采用引擎自带的轻量拼音分支，不加载 G2PW 多音字模型，以降低系统内存占用；少数多音字的准确性可能下降。内存充足且需要完整中文注音时，可在私有 launcher JSON 设为 `"standard"` 后重启面板。直接运行推理脚本对应 `--memory-mode low|standard`。两种模式均保留 CUDA 半精度和单实例推理，外部引擎文件不修改。

启动器会检查 Linux/WSL 和 Windows 主机的可用内存（以及 Windows 可提交余量）；不足 1024 MiB（1 GiB）时拒绝启动 TTS，运行中任一余量低于 768 MiB 时停止由本启动器管理的 TTS，不自动重启。检查约每 5 秒一次，不接管外部服务，也不修改系统内存或交换文件设置；它无法保证避免所有显存不足、驱动故障或黑屏。16 GB 机器建议先使用云端 LLM、单个 TTS 与 VTS，避免同时加载本地 LLM 或训练。

参考录音中的人声、参考文本和所选语言必须逐字对应，不能填写占位数字；上传后即可直接试听，无须先训练。上传音色的目标播报文本单独自动识别语言，不沿用参考录音的语言。TTS 返回完全静音的 WAV 时会显示失败，避免静音任务被误报为播放完成。

声音训练默认只需提供语音素材，由后台自动提取文本后微调；也可选择输入并校对文本，手动填写或提取空白片段的文本，确认后训练。自动提取使用训练 Python 环境中的 `faster-whisper`，在 CPU 上以 int8 运行。在“环境与模型”按用途筛选语音识别，推荐下载 Whisper large-v3-turbo 的 CTranslate2 权重；也支持 Whisper large-v3。页面选择独立保存在 `config/local/asr-model-selection.json`，优先于旧配置，并在下一次转写时读取，无需重启主服务；进行中的整批识别仍使用启动时加载的模型。未保存页面选择时，依次使用 `[training].asr_model` 指定的绝对目录、项目 `data/models/faster-whisper-large-v3-turbo`、旧引擎 `engine_root/tools/asr/models/faster-whisper-large-v3`。选择失效或文件缺失时明确失败，不静默切换模型。识别依赖使用 `[training].python`，与 TTS 的 Python 路径可以不同。其他识别模型仅按用户点击下载并标注“需适配”，不会自动安装或启用。模型目录须包含 `model.bin`、`config.json`、`tokenizer.json`、`preprocessor_config.json`，运行时不会下载。模型语言能力需覆盖所选片段语言；缺少依赖、模型或识别结果为空时会失败，不能用空文本继续训练。手动填写完整文本的流程不依赖识别模型。

参考音频必须由引擎所在 Linux 可访问。M5 的 `[training]` 默认关闭；启用时 `python`、`engine_root` 须为绝对路径，`directory` 相对于配置文件，`timeout_seconds` 为 60–86400 秒。任务在非直播、所选 GPU 显存释放且本地 LLM 停止后运行；受管 TTS 明确报告模型已卸载时可保留其服务端口，否则仍须停止 TTS，项目不停止其他软件的进程。训练输出和缓存只写自有目录，外部引擎安装只读。

试听与版本启用要求 `managed_inference=true`、默认 v2 GPT/SoVITS 两个权重的绝对路径，并使用 `scripts/start-managed-inference.py` 创建的独立推理实例；该实例使用私有可写配置。面板的 TTS 开关已使用此脚本，手动启动方法见脚本的 `--help`。每次合成按音色切换成对权重。服务启动时仅提供 API，模型默认为关闭；在“训练与离线”点击“启用语音模型”后才加载权重。关闭模型会释放推理模型及相关缓存，保留权重文件和已选音色；再次启用后，下次合成仍按已选音色加载权重。此开关同时控制默认模型和训练模型的推理驻留，不控制训练任务的暂停续训。更新后需要重启主服务与受管 TTS 才能使用新接口；旧实例会显示不支持独立开关。未启用训练时保留既有参考音色推理能力；训练素材要求见 [新手说明](../README.md#想进一步训练声音)。2026-09-17 本机 RTX 4050 Laptop（6141 MiB 显存）通过真实训练接口完成 SoVITS 与 GPT 各 5 轮，约 11 分 10 秒生成成对权重并通过校验，任务达到 100%。验收使用重复的参考音频与已校对文本，仅覆盖训练流程，未验证该模型的试听效果；该次验收未覆盖 ASR；仅语音模式需先在“环境与模型”下载并选择识别模型。

训练性能参数随每次任务保存：`batch_size` 为 1–16，`data_workers` 为 0–8，`cpu_threads` 为 1–16，`gpu_index` 为 0–15，`low_memory` 为布尔值。默认依次为 1、1、2、0、true；未包含参数的旧任务使用这些默认值。页签中提供省内存（1/1/2）、均衡（2/2/4）、高吞吐（4/4/8）和自定义设置。GPU 编号选择一张物理显卡，并非多卡并行；当前训练仍要求 CUDA 和 FP16。小数据集的 GPT 阶段保留上游批大小保护，实际批大小可能小于所选值。训练前检查选中 GPU 的占用不超过 512 MiB；参数选项未做多种硬件实测。

仅使用参考音色时无需开启训练或配置权重管理，也可以通过受管 TTS 的专用接口启停默认模型。模型状态接口为 `GET /api/training/models`，控制为 `POST /api/training/models`，请求 `{"enabled":true}` 或 `{"enabled":false}`。加载、卸载、合成和训练有互斥保护；页面断开不会提前释放正在执行的操作。卸载模型后进程及 CUDA 上下文仍可能占用基础内存，停止 TTS 服务才会退出整个进程。


示例中 VTS 默认关闭；面板启动 Windows 执行端前会将实际使用的 `target/windows-client/desktop.local.toml` 中 `[vtube_studio].enabled` 写为 `true`，无须修改示例文件。直接启动客户端时仍需手动启用。启用后，客户端独立连接 `websocket_url`，首次由 VTS 弹窗授权。`token_path` 相对于 TOML 所在目录解析，默认保存到 `config/local/vts-token.json`；授权文件不进入前端或日志。Unix 文件权限须为 `0600`，Windows 应保存在当前用户的私有目录。拒绝、撤销授权或授权超时后不会反复弹窗；确认 VTS 设置、移走失效的本地授权文件并重启客户端后重新授权。已有授权文件不会自动覆盖。

客户端创建范围 `0..1` 的 `MeowMouthOpen` 自定义输入；在 VTS 的模型参数配置中手动映射到该模型的嘴部开合参数。可通过 `mouth_parameter` 改名（4–32 位 ASCII 字母或数字）。嘴部映射未配置时，即使客户端显示 `Connected`，模型也不会自动张嘴。插件授权与连接状态目前输出在桌面进程日志中。

`[lip_sync]` 使用设备输出样本的 RMS：`noise_floor` 为静音阈值（0–0.5），`gain` 为放大系数（0.1–20），`attack_ms`/`release_ms` 为开口/闭口平滑时间（1–1000 / 1–2000 毫秒），`update_hz` 为观测频率（10–30）。VTS 常规注入最多 30 次/秒，仅保留最新值；停止立即发布零值，超过 250 毫秒未更新的值按静音处理。设备延迟前、任务完成、失败和连接断开时均复位。VTS 故障独立于播放连接，不影响停止回执；目标应用失联时无法保证立即改变画面，重连会先发零值。

Agent 启动始终暂停。`[agent]` 的 `persona` 为 1–2000 个 Unicode 字符，`topic` 可空、最多 200 字；`cooldown_ms` 为 1000–3600000 毫秒，成功、失败、忽略及播放终态都会进入冷却。只有 `proactive_enabled=true` 且无事件、无播报时才会主动发言，首次恢复后也要等待冷却。

调度上限为 `pending_capacity` 1–512、终态 `history_limit` 1–2000、`dedup_capacity` 1–4096、`batch_size` 1–16、已播完 `conversation_limit` 1–20 轮；`event_ttl_ms` 为 1000–600000，`gift_merge_ms` 为 0–10000（0 禁用分组）。去重 ID 在会话内全局唯一，平台适配器须给原始平台 ID 加来源前缀；活动 ID 不因去重缓存淘汰而失效，终态 ID 超出缓存后可被重新接收。默认内存模式不提供跨重启去重；启用下文 PostgreSQL 持久事件后，数据库会阻止已接收事件在重启后重复入队。

`[llm]` 的 `base_url`/`model` 同时留空可保留人工播报模式；填写后会在启动时校验并读取 `api_key_env` 指定的环境变量。环境变量缺失或空值时 Agent 保持未配置，主服务继续运行以便从面板修复；`api_key_env=""` 显式选择无认证模式。此适配器直连配置地址，不自动继承 HTTP 代理、不跟随重定向，错误不公开响应正文或密钥。模型名最多 128 字节，`max_tokens` 为 64–65536（本地模式仍不超过 1024），`max_response_bytes` 为 1024–1048576。`timeout_seconds` 为 1–120 秒，覆盖一轮全部尝试及重试等待；`max_retries` 为 0 或 1，只重试超时、连接失败、429 和 5xx 等临时错误。

“LLM 接入”页使用 **填写 API 地址和密钥 → 获取模型 → 下拉选择 → 设置推理强度 → 测试连接 → 保存配置** 的流程，不再要求手工输入模型名称。常见官方地址自动选择服务商和 API 格式；未知供应商默认使用 OpenAI Chat Completions。高级设置保留 GPT、Claude、Kimi、GLM、Grok、Gemini、DeepSeek 预设、API 格式、运行模式、超时、输出上限和 JSON 选项。供应商使用特殊协议时，可展开调整为 `openai_chat`、`openai_responses`、`anthropic_messages` 或 `gemini_generate_content`。JSON 模式不受支持时可关闭；所有格式仍校验 Agent 决策。

模型列表直接从填写的供应商 API 获取，不抓取网站页面或内置固定模型名称。地址支持 API 根地址、含版本路径的地址和完整推理端点；不能含查询参数、凭据或片段。获取成功后使用规范化的 API 根地址继续测试和保存。OpenAI 兼容目录使用 `/models`，Anthropic 与 Gemini 使用各自协议并处理分页；Gemini 只显示声明支持 `generateContent` 的模型。OpenAI 兼容目录通常不提供完整能力信息，出现在列表中不保证支持本项目的结构化回答，仍需测试所选模型。协议参考：[OpenAI 模型列表](https://developers.openai.com/api/reference/resources/models/methods/list)、[Anthropic 模型列表](https://platform.claude.com/docs/en/api/models/list)、[Gemini 模型列表](https://ai.google.dev/api/models)。

获取列表仅使用当前草稿，不保存配置、不生成回答。连接信息改变时旧列表与选择失效；已有保存的模型可继续显示和使用，主动刷新后若模型已不在目录中则须重新选择。空列表、不支持列表接口、鉴权失败或超时会明确提示。供应商没有可用列表接口时无法通过此页面新增选择，不会用猜测的模型名称替代。目录请求总超时 30 秒，累计响应最多 1 MiB、最多 10 页和 1000 项；超过上限明确失败，不静默展示不完整列表。

`GET /api/llm/settings` 读取待启用设置与密钥存在标记，`POST /api/llm/models` 获取连接草稿的模型目录，`POST /api/llm/reasoning` 在本地解析模型推理档位，`POST /api/llm/settings` 保存当前配置，`POST /api/llm/profiles` 新建配置，`POST /api/llm/profiles/select` 切换配置，`POST /api/llm/profiles/rename` 重命名，`POST /api/llm/profiles/delete` 删除，`POST /api/llm/test` 对当前草稿的所选模型执行一次连接测试。配置面板支持保存多组服务商连接；保存会覆盖当前项，另存为会保留当前项并创建新项，标题可单独重命名。首次保存默认名为“配置1”，后续自动编号；每项各自保存连接和密钥。切换所选项会保存当前选择，重启主服务后生效，页面会提示待重启状态。旧版单项配置会迁移为“配置1”。测试不保存且不播放声音。面板保存位置为 TOML 同级 `local/<文件名去掉扩展名>-llm.json`，例如 `config/local/server.local-llm.json`，优先于 `[llm]`；密钥以本机明文保存，Unix 文件权限 `0600`，查询及日志不公开密钥。移走该文件并重启可恢复 TOML 配置。密钥留空仅保留相同服务商、格式和规范化 API 地址的密钥；根地址与 `/v1` 等不同路径仍视为不同连接，更换目标必须重新填写或明确清除。复用已保存密钥时只查询原 API 路径下的模型目录，不跨路径自动探测。模型目录和连接测试共享单个请求名额，目录请求不跟随重定向、不使用环境代理、不公开供应商错误正文。

`mode` 默认 `cloud`；设为 `local` 时 LLM/TTS 地址必须使用回环 IP，`api_key_env` 必须为空，`max_tokens` 不超过 1024，`max_retries=0`。这些约束不等于完整离线验收；面板测量真实 LLM→TTS 的耗时及显存，并在重启、训练或启用新版本后清除旧验证标记。

Agent 使用统一运行层处理四种原生协议的流式输出、只读工具、缓存与用量；最终仍以完整 JSON 决策生成一条语音：`reply_to` 是本轮候选 ID 子集，`text` 为 1–500 字或 null，`topic` 为最多 200 字或 null。多余动作字段、截断、未知 ID 和礼物组部分选择会失败；模型工具限于启用的时间、环境、OBS 状态和网页检索，不执行任意控制命令。输入事件、工具结果与历史作为未受信资料，人设才进入 system 指令。运行设置、搜索配置和费用估算见 [Agent 运行说明](agent-runtime.md)。连接测试保留完整非流式决策验证；兼容协议不等于已验证每家提供商，首次真实调用前应使用小批事件核对响应与模型计费。

Agent HTTP 接口为 `GET /api/agent`，`POST /api/agent/settings`、`/api/agent/pause`、`/api/agent/resume`，以及 `POST /api/events`（`{"events":[...]}`，每批 1–100 条，256 KiB 上限，整体校验后接收）。这些入口沿用主服务来源校验；原 `/api/stop` 会同时暂停 Agent。

面板点击“保存设置并暂停”后，人设、系统提示词、话题、冷却时间、主动发言选项和互动策略保存到 TOML 同级 `local/<文件名去掉扩展名>-agent.json`，例如 `config/local/server.local-agent.json`。保存成功即生效，重启时优先于 `[agent]` 的对应字段和 `[agent.interaction]`；队列容量等调度上限仍读取 TOML。旧文件缺少 `system_prompt` 或 `interaction` 时使用默认值。文件原子替换，Unix 权限为 `0600`，保存失败保留当前设置并向面板报错。未保存的草稿不写入文件。重启恢复配置但保持暂停，不恢复旧的待播事件。移走该文件并重启可恢复 TOML 配置；文件损坏时启动会报错，不会静默覆盖。

### 弹幕、SC与欢迎策略

在 **Agent 互动 → 弹幕与欢迎** 配置，也可设置 `[agent.interaction]`：

| 字段 | 默认 | 行为与范围 |
| --- | --- | --- |
| `chat_read_mode` | `auto` | `auto` 稀疏逐条、繁忙挑选；`all` 始终逐条；`selective` 始终允许模型挑选 |
| `welcome_enabled` | `true` | 开启符合条件时的点名欢迎 |
| `busy_chat_count` | `6` | 最近 60 秒收到的有效普通弹幕数，1–1000 |
| `busy_enter_count` | `3` | 最近 60 秒收到的有效进房数，1–1000 |
| `busy_pending_count` | `4` | 当前待处理事件数，1–512 |
| `welcome_cooldown_ms` | `30000` | 全局欢迎间隔，1000–3600000 毫秒 |
| `welcome_viewer_cooldown_ms` | `600000` | 同一观众欢迎间隔，1000–86400000 毫秒 |

`[agent]` 的 `system_prompt`（默认空字符串，最多 4000 个字符）可在 Agent 互动页编辑。它作为用户自定义系统提示词发送给模型，适合补充语气、互动规则和回复偏好；固定 JSON 输出、事件选择、工具权限和数据处理规则仍由程序维护。

三个繁忙阈值达到任一个就启用繁忙策略；最近 60 秒为滚动窗口，重复和已过期事件不计入。当前官方接入没有可靠的实时在线人数，不把热度、累计观看量当成人数；大房间即使暂时安静，也可手动关闭欢迎。

逐条模式每轮直接读原始弹幕，再说模型回复；挑选模式只朗读被选中的原文并回复。普通弹幕不会自动添加“观众昵称说”前缀，SC 只保留感谢昵称和金额后直接读正文。模型返回合法的“不回复”时，逐条弹幕、SC 和欢迎仍有固定播报兜底。模型报错、事件过期、暂停、停止和容量限制仍可能阻止播报，“逐条”不等于无条件保证全部播完。

SC 接收 `LIVE_OPEN_PLATFORM_SUPER_CHAT`，保存人民币元金额、完整正文和 UTC 毫秒起止时间。每条 SC 优先单独处理，先说“感谢昵称的金额元SC”，读正文，再回应留言；有效期使用平台展示结束时间，不套普通弹幕 TTL。收到时或模型返回时已过期就不再开始排队；已经排入语音队列的播报允许完整结束，不在展示到期时截断。重传按房间和 SC `message_id` 去重。SC 撤回通知 `LIVE_OPEN_PLATFORM_SUPER_CHAT_DEL` 尚未接入，撤回不会自动取消待播或正在播放的内容。

进房接收 `LIVE_OPEN_PLATFORM_LIVE_ROOM_ENTER`，欢迎包含昵称与感谢；每轮只欢迎一人，全局和同人冷却从安排播报开始计时。没有稳定身份时以来源和昵称保守去重。繁忙、关闭欢迎、冷却中或有弹幕/礼物/SC 待处理时直接跳过欢迎；生成期间变忙也会跳过。进房事件最多保留 15 秒待响应，避免读到过时欢迎。

队列满时，高优先级事件可替换尚未处理的低优先级事件，顺序为 SC、礼物、普通弹幕、进房；替换有明确跳过原因。同级队列已满仍会拒绝接收。普通礼物继续沿用分组合并和互动公平性规则；SC 与进房保留原始事件，但不计为普通礼物积分。

弹幕、SC 正文和模型回复各最多 500 字；程序组合的“朗读原文＋回复”最多 1200 字，候选原文总长度受限，超出的事件留待下一轮。人工输入播报仍最多 500 字。长语音资源上限见下文；请在事件历史与语音队列检查失败原因，只有设备返回 `completed` 才算完成播报。

长语音的默认合成超时为 300 秒，`[speech].max_audio_bytes` 默认及最大为 33554432（32 MiB WAV），桌面 `max_buffer_samples` 默认及最大为 16777216 个输入样本。Windows 重采样后的实际浮点缓冲仍有 128 MiB 上限并按需分配；过长音频会明确失败，不能用文本长度推算保证播放时长。已有本机 TOML 若显式写了旧的 8388608 字节或 5760000 样本，需分别调整服务端与桌面配置并重启才会使用扩大后的容量；不会自动改写私有配置。真实语速、硬件输出格式与合成耗时仍需实机验证。


**推荐直接在控制面板的「直播连接」页设置**：启用直播接入，填写应用 ID、AccessKey ID、AccessKey Secret 和主播身份码，点击保存后即可连接，无需编辑文件、设置环境变量或重启主服务。应用 ID 须为 1–9223372036854775807，页面以字符串提交以避免大整数精度损失。密钥框留空保留已保存值；更换应用 ID 必须重新填写凭据；清除凭据须同时关闭接入。连接进行中或断开清理尚未完成时拒绝修改配置。

`GET /api/live/settings` 仅返回设置及凭据存在标记，`POST /api/live/settings` 校验后原子保存到 TOML 同级 `local/<文件名去掉扩展名>-live.json`，例如 `config/local/server.local-live.json`。Unix 文件权限为 `0600`，查询和调试输出不包含凭据，保存失败保留原设置。下次启动自动读取本机覆盖文件。未通过面板保存时仍兼容 `[live]` 及 `access_key_id_env`、`access_key_secret_env`、`identity_code_env` 指定的环境变量；这些兼容入口不再是普通用户的必填步骤。

启动不自动授权，面板“连接直播间”才调用项目开始接口；官方授权响应决定房间号。

`reconnect_initial_ms` 为 10–60000 ms，`reconnect_max_ms` 不小于初始值且不超过 300000 ms；默认从 1 秒指数退避至 30 秒。临时网络错误进入重连，永久鉴权或协议错误进入失败；清理项目失败会停止自动重试并展示错误，避免连续创建无法核对的会话。手动断开期间拒绝新连接，直至旧会话清理结束。平台失联暂停 Agent，恢复连接不自动恢复模型调用。连接状态和即时计数仅在内存保存；启用持久事件后，收到的观众事件另外保存到 PostgreSQL。

## M6 桌面与 OBS

Windows Tauri 启动方法见 `apps/desktop/src-tauri/README.md`。主服务的 `allowed_origins` 须包含 `http://tauri.localhost` 以及开发面板地址；桌面 `server_url` 填主服务 HTTP 或 WS origin，不填 `/api` 路径。原生启动地址优先于前端构建变量。跨 WSL 访问时同时调整主服务监听地址与桌面连接地址。

**推荐直接在「OBS 场景与录制」页设置**：先在 OBS 的“工具 → WebSocket 服务器设置”中启用服务，再在面板启用 OBS 控制，填写地址（默认 `ws://127.0.0.1:4455`）和密码后保存。保存由 Windows 执行端完成，后续操作立即使用新设置，重启后自动读取；执行端未连接时需先启动它。密码框留空保留已保存值，可以显式清除；更换地址不会默认复用旧密码。

`GET/POST /api/obs/settings` 通过桌面资源通道读取和保存；私有文件位于执行端配置旁，例如 `target/windows-client/desktop.local.obs.local.json`。查询只返回密码存在标记。Unix 权限为 `0600`，Windows 使用所在目录的访问权限；无密码不代表 OBS 已连接，保存后可在面板测试连接。

高级用户仍可使用执行端私有 TOML 和环境变量作为初始配置：

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

## 启用观众档案与持久事件

源码配置的 `[viewers].enabled` 默认是 `true`；Windows App 首次生成的独立配置为 `false`，需按 [补齐步骤](windows-database.md) 准备数据库再启用。`[auth].enabled` 默认是 `false`；控制面板用户就是软件管理者，可以直接查看和管理观众、事件、陪伴积分、记忆及关系。`scope_id` 表示稳定逻辑角色，重启、重新开播、换音色或 Live2D 外观时保持一致。旧配置若显式写了 `viewers.enabled=false` 或 `auth.enabled=true`，分别改为 `true` 和 `false`，重启主服务即可使用默认体验。

通过面板或 `npm run start:server -- --config config/server.local.toml` 启动时：

1. 若已提供 `MEOWLIVE_DATABASE_URL`（或 `database_url_env` 指定的自定义变量），直接连接该数据库，不启动本地容器。
2. 未提供默认连接变量时，读取本项目 `config/local/databases.env`；全新安装自动生成随机密码。已有 PostgreSQL 数据却缺失凭据时会要求恢复原凭据，不生成新密码覆盖旧库。
3. 启动本项目 Compose 的 `postgres` 服务，等待就绪后，使用受限的 `meowlive_app` 账号连接 `127.0.0.1:25432/meowlive`。应用连接只注入主服务环境，不返回前端或传给 TTS。
4. 主服务检查数据库连接并执行版本化迁移，然后开放观众、事件、积分和记忆接口。缺少 Docker 或连接失败会显示错误，不静默退回临时内存存储。关闭主服务不会删除或停止数据库。

直接运行 Rust 二进制或 `rust_cache.py run` 时，需要自行启动数据库并提供私有连接变量。若显式设置 `[viewers].enabled=false`，可使用不依赖 PostgreSQL 的临时模式。密码含特殊字符时，手工构造连接 URL 须进行 URL 编码；自动准备入口已处理编码。

单独部署需要口令保护的服务时，可以显式设置 `[auth].enabled=true`，通过 `admin_token_env` / `admin_token_file` 和 `device_token_env` / `device_token_file` 分别提供不同的 32–512 字节 ASCII 凭据；文件路径相对主服务 TOML，环境变量优先。此模式会要求面板登录，Windows 执行端通过独立 `device_token_file` 认证。默认单用户控制面板无需准备这些口令。

`GET /api/health` 返回服务标识、协议版本和执行端连接状态；`/api/admin/session` 查询访问模式，默认返回 `enabled=false, authenticated=true`。显式认证模式额外支持 POST 登录和 DELETE 注销。观众与事件查询入口为 `/api/admin/viewers` 与 `/api/admin/events`，分页参数 `limit` 为 1–100，`offset` 为 0–1000000。它们不接受客户端指定其他 scope。

数据库先保存事实，再尝试进入有界回应队列。HTTP 接收结果中的 `persisted` 是本次新落库数量，`accepted` 是安排回应数量，`duplicates` 是已有事件，`unscheduled` 是已落库但未安排回应的数量。直播中断和数据库失败不保证平台重放，事件页的 `unconfirmed_events` 记录本次服务运行期间的未确认接收数量；服务重启会重置此诊断计数，不能用 0 推断无历史缺口。服务重启不会恢复旧事件为待播报。

HTTP 模拟/回放输入在持久化模式中强制隔离为 `simulator` 来源和命名空间，不能伪造 B 站真实档案。B 站有效 UID 使用字符串存储；缺 UID 时仅在已知应用 ID 的命名空间中使用 open_id。没有稳定身份只保存匿名事件，不创建观众档案。昵称不作为身份依据。礼物原始 price/paid/等级字段保留，不推算已支付总额或自动积分。

集成验证需显式为专用测试数据库设置 `MEOWLIVE_TEST_DATABASE_URL`，再运行：

```bash
python3 scripts/rust_cache.py test -p meowlive-adapters --test postgres_viewers --locked -- --ignored
python3 scripts/rust_cache.py test -p meowlive-server --test viewer_process --locked -- --ignored
```

普通 `npm run check` 会把这两类真实数据库测试标为 ignored；不能把该状态算作数据库验证通过。查询返回最近 100 个历史昵称和最多 100 个身份；完整事实仍保存在 PostgreSQL。管理员可在观众详情中管理积分、记忆和关系，并预览和确认身份合并。真实 B 站 open_id 跨场稳定性、礼物金额语义和 Windows 声音仍需授权实机验收。


## 陪伴、记忆与图谱

`[viewers].calendar_offset_minutes` 控制日历日边界，默认 `480`（UTC+8），允许 -840 至 840，不跟随主机时区或夏令时。熟悉度按真实观察到的来访日去重；有效交流奖励只在执行端回报 completed 后入账，同正文和原事件不重复奖励。失败、取消、断线或尚未确认的播报不奖励。礼物保留平台原值，只有管理员提供依据确认的人民币分值才参与有限积分；没有真实平台样例时不推测价格单位。

设备完成回执在状态机确认前写入 `[viewers].receipt_directory` 本地同步日志，再异步提交数据库。目录相对 TOML 所在目录，必须保留跨重启、仅服务用户可写；日志只记录 scope、speech ID、完成时间及失败尝试，不保存对话。最多 4096 条，接近上限时暂停新一轮生成；失败按尝试次数轮转，不由一条坏记录阻塞后续回执。服务重启会恢复未入账的真实完成回执，数据库幂等键阻止重复奖励；不根据“已生成”猜测播放完成，也不重播历史内容。磁盘写入失败时断开设备并显示失败诊断，不能伪造已持久状态；设备回执到达前的网络中断仍属于未确认。管理页展示失败 speech ID，排查时保留原日志，不人工补造回执。

PostgreSQL 记忆存储随 `[viewers]` 默认开启，可直接查询、纠正、冻结和删除已保存的记忆。`[memory].enabled` 仅控制独立模型的后台提取，默认关闭；开启时配置独立的 OpenAI 兼容接口 `endpoint`/`model`；`api_key_env` 仅由服务端环境读取，空字符串明确表示无认证。可选嵌入配置使用独立的 `embedding_endpoint`、`embedding_model` 和 `embedding_api_key_env`；模型及维度共同隔离向量。接口使用 chat/completions JSON 和 embeddings，不授予模型工具或数据库执行权限。请求和响应有 64 KiB 限额，`timeout_ms` 为 1–60000。缺少嵌入或请求失败时仍可检索结构化事实。

后台提取并发为 1，每次取一个有 60 秒租约的持久任务，最多尝试 3 次。Agent 正在生成、说话或本地 GPU 被占用时不开始下一次提取；独立外部模型的实际 GPU 调度仍由部署方负责。已接受但未安排回应的聊天也进入提取队列。候选默认 7 天；临时状态默认 24 小时；共同经历有 7 天半衰期、30 天硬期限；长期事实不会因沉默自动删除。自动确认采用保守的中文明确自述模式，其他说法保留候选，不能把规则测试当作真实模型的召回率验收。 HTTP 提取结果自报绝对期限时会拒绝该候选，尚未接入可核对的自然语言日期解析；也未自动将“重要共同经历”晋升为长期事实。

管理纠正会锁定正文；显式解冻后允许新来源更新，但原来源和删除标记继续抑制旧内容。纠正、删除、过期及合并会使旧上下文失效，并取消在途或待播任务；已经播放的部分不能撤回。后台向量提交必须符合模型、维度、正文版本及当前租约。每轮只注入当前观众的最多 8 条有效记忆、3 条已确认关系，文本合计最多 1200 UTF-8 字节，作为保守 token 上界；分数和管理审计不进入模型或执行端。

`[graph]` 开启后以环境变量提供 Neo4j 密码。适配器对应 Compose 固定的 Neo4j 5.26.30 Query API；关系事实和同步 outbox 在 PostgreSQL 中同时提交。图服务断开时后台重连，最多 3 次投影尝试，失败可从管理界面重建。图返回 ID/版本必须再经 SQL 核对有效状态与连通路径；故障降级为 SQL 的有界一跳查询。已确认的明确兴趣和经历可在记忆事务内派生话题/活动关系；“我认识…”或“我和…是朋友”的明确原文仅派生未解析提及，单方声称保持未确认，昵称不自动解析为另一位稳定观众；管理员可以核对原始事件证据后确认或撤销。

身份合并必须先生成预览，再提供理由并明确确认。预览期间任何相关事实变化均应重新预览。合并保留来源身份和历史流水，同日来访去重；好感采用双方当前值的较大值，避免两份同人档案直接相加，互补历史可能被低估，可通过有依据的人工调整补正。合并前的旧流水不再直接撤销，需要以合并后分值为基准填写有理由的人工调整；合并后的新调整仍可正常撤销。旧来源档案留合并记录并退出普通列表，新增事件归入目标身份。

管理恢复接口均需管理员认证和唯一 `request_key`、非空 `reason`。网络失败重试应复用相同请求键和完全相同的参数；修改参数须新建请求键：

| 操作 | 接口 |
| --- | --- |
| 提取/嵌入失败重试 | `POST /api/admin/memories/retry` |
| 从有效记忆重建向量 | `POST /api/admin/memories/rebuild-vectors` |
| 从关系事实和删除记录重建图 | `POST /api/admin/graph/rebuild` |
| 任务诊断 | `GET /api/admin/memories/status`、`GET /api/admin/graph/status`、`GET /api/admin/companionship/status` |

原始聊天及提取输入默认保留 30 天；长期记忆仅保留必要证据片段和来源 ID。礼物账本、事件去重依据、积分流水、抑制和删除记录不随聊天清理。

数据库备份和恢复步骤见 [观众数据恢复](viewer-recovery.md)。除常规检查外，真实存储验收需专用 `MEOWLIVE_TEST_DATABASE_URL`；图测试还需 `MEOWLIVE_TEST_NEO4J_PASSWORD`，地址为本项目回环端口 `17474`。测试只使用随机 scope，普通 `npm run check` 中 ignored 项不代表已验证。
