# 开发命令

当前命令集中在根 `package.json`：

- `./launchers/start.sh`：新人单入口，必要时安装前端依赖，打开带服务滑动开关的控制面板。
- `npm start`：启动同一个面板与本机服务管理器；`-- --no-open` 不自动打开浏览器。
- `npm run dev`：仅启动开发网页，主服务和 TTS 需自行管理。
- `npm run test:launcher`：验证配置、密钥隔离、HTTP 控制边界及实际子进程启停、取消、冲突与回收。
- `npm run typecheck`：检查所有 TypeScript 工作区。
- `npm run check:rust`：检查 Rust 格式与 workspace 编译。
- `npm run tree:update`：根据用途登记生成每个受维护目录的递归索引。
- `npm run tree:check`：检查缺少用途、残留条目、缺失索引和内容变化。
- `npm run test:tooling`：在临时目录验证索引生成和检查行为。
- `npm run check`：执行目录、工具、Rust 编译/测试、协议生成一致性、TypeScript 与前端组件检查。
- `npm run build`：类型检查并构建控制面板。

`directory-descriptions.json` 保存可提交工程的用途说明，`directory-tree.mjs` 负责扫描、生成和校验，`directory-tree.test.mjs` 验证实际文件增删改行为。`docs/` 使用独立的本地用途登记文件，避免已忽略文档影响其他检出。

脚本不解析业务内容；修改者负责说明准确性。它只在全部用途通过校验后写入索引，检查模式不写文件，重复生成且文件未变化时不重写。

`npm run contracts:generate` 从 Rust DTO 生成 TypeScript，`contracts:check` 核对一致性。`start:server` 和 `start:client` 启动对应 Cargo 二进制。业务规则保留在对应模块。

`start-control-panel.mjs` 将 `launcher/` 中的配置、进程管理和 HTTP 控制接入 Vite。只接受 loopback 同源面板、会话令牌和固定的主服务、TTS 和 Windows 执行端三个开关，不接收任意 shell 命令；入口不自动启动模型或业务服务。主服务始终为现有 Rust 服务，Windows 执行库与协议版本不变。管理状态契约由 `crates/protocol/src/launcher.rs` 生成。真实服务日志和推理工作目录分别位于已排除索引的 logs/control-panel 与 data/control-panel-inference。

M5 引擎工具：`train-gpt-sovits.py` 读取服务端生成的任务清单，逐阶段调用上游 v2 训练入口；`engine_workspace.py` 创建只在项目数据内写入的源码副本、缓存及受限环境；`start-managed-inference.py` 启动使用私有 YAML 的本机推理实例。运行脚本应使用 GPT-SoVITS 对应 Python，外部安装保持只读。`training_test.py` 用标准库测试任务与路径、代理环境和低显存兼容配置；`npm run check` 包含这些测试。Python 缓存被 Git 与目录索引共同排除。

M6 观测工具：`npm run acceptance:observe -- --duration-seconds 600 --execution simulated` 只查询主服务状态并保存脱敏 JSON；`--observe` 是显式只读模式，默认也是只读。输出文件必须不存在。`npm run test:acceptance` 使用受控 HTTP 服务检查采样、超时、限流量、历史基线和重启计数；已纳入 `check`。Tauri 自动生成的 `src-tauri/gen/` 整体排除索引，避免生成 schema 的父目录触发用途缺项。

Windows 开关由 `launcher/windows-client.mjs` 管理固定 Windows helper 作业与主服务握手状态；`launchers/windows-client.ps1` 使用 Windows Job Object 持有真实执行端，退出或租约过期时回收。日常入口集中于根 `launchers/`。
