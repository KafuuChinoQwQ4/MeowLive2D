# 开发命令

当前命令集中在根 `package.json`：

- `./launchers/start.sh`：新人单入口，必要时安装前端依赖，打开带服务滑动开关的控制面板。
- `npm start`：启动同一个面板与本机服务管理器；`-- --no-open` 不自动打开浏览器。
- `npm run dev`：仅启动开发网页，主服务和 TTS 需自行管理。
- `npm run test:launcher`：验证配置、密钥隔离、HTTP 控制边界及实际子进程启停、取消、冲突与回收。
- `npm run typecheck`：检查所有 TypeScript 工作区。
- `npm run check:rust`：检查 Rust 格式与 workspace 编译。
- `npm run build:rust`：构建 Rust workspace，成功后回收被替换的旧产物。
- `npm run rust:cleanup -- --dry-run`：预览与当前已记录配置相同的旧可执行程序；去掉 `--dry-run` 执行清理。
- `npm run test:rust-cache`：验证当前缓存保护、失败构建、Cargo 文件锁及真实重复构建的缓存命中。
- `npm run tree:update`：根据用途登记生成每个受维护目录的递归索引。
- `npm run tree:check`：检查缺少用途、残留条目、缺失索引和内容变化。
- `npm run test:tooling`：在临时目录验证索引生成和检查行为。
- `npm run check`：执行目录、工具、Rust 编译/测试、协议生成一致性、TypeScript 与前端组件检查。
- `npm run build`：类型检查并构建控制面板。

`directory-descriptions.json` 保存可提交工程的用途说明，`directory-tree.mjs` 负责扫描、生成和校验，`directory-tree.test.mjs` 验证实际文件增删改行为。`docs/` 使用独立的本地用途登记文件，避免已忽略文档影响其他检出。

脚本不解析业务内容；修改者负责说明准确性。它只在全部用途通过校验后写入索引，检查模式不写文件，重复生成且文件未变化时不重写。

`npm run contracts:generate` 从 Rust DTO 生成 TypeScript，`contracts:check` 核对一致性。`start:server` 和 `start:client` 启动对应 Cargo 二进制。业务规则保留在对应模块。

Rust 开发入口通过 `rust_cache.py` 调用稳定版 Cargo，保留原有 incremental、调试信息与编译参数。成功构建后根据 Cargo JSON 消息记录实际使用的产物（包括 `fresh` 命中的旧文件），同一命令下次成功后回收被替换的可执行程序和已准确记录归属的旧增量目录。`check`、`test`、单包运行等命令的当前产物取并集保护，不以修改时间或保留天数判断有效性。失败构建不更新清单也不清理；清理持有 Cargo `.cargo-lock`，存在其他构建时跳过。

首次 `rust:cleanup` 只清理与已成功构建的配置指纹相同的历史可执行程序，保留依赖库、构建脚本、工具链和无法准确确认归属的历史增量目录。`target/.rust-cache/` 内的清单属于可丢弃构建状态；缺失时保守跳过，损坏时报告错误。清理统计为去重后的文件分配空间估算，Windows 上不支持 Unix 文件锁时跳过删除。直接运行裸 `cargo`、Tauri 自身构建或自定义输出目录不会自动维护此清单；日常使用上述 npm 命令。切换配置、修改依赖或删除有效缓存仍会产生必要的重新编译，清理不承诺任意配置永远零重编译。

`start-control-panel.mjs` 将 `launcher/` 中的配置、进程管理和 HTTP 控制接入 Vite。只接受 loopback 同源面板、会话令牌和固定的主服务、TTS 和 Windows 执行端三个开关，不接收任意 shell 命令；入口不自动启动模型或业务服务。主服务始终为现有 Rust 服务，Windows 执行库与协议版本不变。管理状态契约由 `crates/protocol/src/launcher.rs` 生成。真实服务日志和推理工作目录分别位于已排除索引的 logs/control-panel 与 data/control-panel-inference。

M5 引擎工具：`train-gpt-sovits.py` 读取服务端生成的任务清单，逐阶段调用上游 v2 训练入口；`engine_workspace.py` 创建只在项目数据内写入的源码副本、缓存及受限环境；`start-managed-inference.py` 启动使用私有 YAML 的本机推理实例；`model_runtime.py` 为自有 API 副本安装延迟加载和显式模型启停，TTS 服务就绪不代表模型已加载。运行脚本应使用 GPT-SoVITS 对应 Python，外部安装保持只读。`training_test.py` 用标准库测试任务与路径、代理环境、性能参数传递和低显存兼容配置；`model_runtime_test.py` 用模拟模型验证启停、清理和请求取消保护，推理 Python 含 FastAPI 时还运行受控 HTTP 测试；`npm run check` 包含这些测试。Python 缓存被 Git 与目录索引共同排除。

训练子进程的临时文件使用项目内的 `data/tmp/gsv-*` 短目录，避免 Python 3.10 数据加载进程的 Unix socket 路径超长。阶段正常结束、失败或收到取消信号时清理该目录；模型缓存仍在任务工作区。`stages.log` 实时写入阶段输出，最多保留 256 KiB；界面百分比按阶段更新（SoVITS 为 40%、GPT 为 70%），阶段内可通过日志中的批次和轮次判断训练是否推进。

同音色续训由应用层选择最近一次成功版本，存储层校验并复制成对权重到新任务的 `base/`，在 `job.json` 中记录来源与 SHA-256。训练器使用这组权重初始化 GPT 和 SoVITS，语义预处理仍使用通用编码器；优化器与本次轮次重新计数。无历史成功版本或旧清单没有 `base` 时沿用通用模型。来源文件损坏时明确失败，不静默从通用模型重训；新任务使用私有副本，删除旧版本不破坏新版本。

训练支持仅提供音频和人工校对文本两种方式。`training_transcription.py` 复用本地 faster-whisper 模型，在 CPU 上提取缺少的文本；`transcribe-training.py` 提供单片转写命令，供面板回填和校对。示例：`python scripts/transcribe-training.py --audio /绝对路径/片段.wav --language zh --engine-root /绝对路径/GPT-SoVITS --model /绝对路径/识别模型`。优先读取页面保存的 `config/local/asr-model-selection.json`（`MEOWLIVE_ASR_SELECTION` 可指定独立选择文件）；没有保存选择时，依次使用 `--model`、`MEOWLIVE_ASR_MODEL`、项目 `data/models/faster-whisper-large-v3-turbo`，最后兼容旧引擎 `tools/asr/models/faster-whisper-large-v3`。选择失效不自动回退。`launcher/model-catalog-asr.mjs` 定义识别模型下载范围，`launcher/model-asr.mjs` 检查识别依赖和本地权重；`asr_selection_test.py` 验证选择优先级及失败行为。识别只使用本地模型，不自动下载；文本校验失败时中止，已手填文本不会被覆盖。

M6 观测工具：`npm run acceptance:observe -- --duration-seconds 600 --execution simulated` 只查询主服务状态并保存脱敏 JSON；`--observe` 是显式只读模式，默认也是只读。输出文件必须不存在。`npm run test:acceptance` 使用受控 HTTP 服务检查采样、超时、限流量、历史基线和重启计数；已纳入 `check`。Tauri 自动生成的 `src-tauri/gen/` 整体排除索引，避免生成 schema 的父目录触发用途缺项。

Windows 开关由 `launcher/windows-client.mjs` 管理固定 Windows helper 作业与主服务握手状态；`launchers/windows-client.ps1` 使用 Windows Job Object 持有真实执行端，退出或租约过期时回收。日常入口集中于根 `launchers/`。
