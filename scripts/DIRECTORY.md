# scripts 目录索引

开发工具、目录用途登记与索引同步检查

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
scripts/  # 开发工具、目录用途登记与索引同步检查
├── launcher/  # Linux 本机服务启动管理、配置读取、状态探测与控制接口
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── config.mjs  # 私有启动配置、默认观众存储元数据及主服务环境的加载与脱敏
│   ├── config.test.mjs  # 启动配置、缺失环境、密钥隔离和本机地址限制的测试
│   ├── database.mjs  # 默认观众存储的私有数据库凭据准备、项目 PostgreSQL 启动和受限应用连接注入
│   ├── database.test.mjs  # 默认数据库首次准备、凭据复用、外部连接隔离和错误脱敏测试
│   ├── health.mjs  # 通过公开最小健康接口检查主服务与 TTS 就绪和端口占用
│   ├── health.test.mjs  # 认证主服务通过公开最小健康接口被启动器和设备探测的测试
│   ├── http.mjs  # 同源会话保护的三个服务开关及模型管理 HTTP 接口
│   ├── http.test.mjs  # 启停接口来源、会话、请求形状及大小限制测试
│   ├── log.mjs  # 受管进程日志限量保存、密钥遮盖及常见启动故障识别
│   ├── log.test.mjs  # 子进程日志跨数据块密钥遮盖与显存错误诊断测试
│   ├── memory.mjs  # 读取 Linux/WSL 与 Windows 主机内存余量，提供受管 TTS 启动和运行保护判定
│   ├── model-asr.mjs  # 识别模型选择读取、Whisper 文件完整性和训练 Python 依赖检查
│   ├── model-catalog-asr.mjs  # Whisper Turbo 与其他可选本地语音识别模型的来源、文件范围和适配状态
│   ├── model-catalog.mjs  # 核对官方来源的公开语音模型目录、下载文件范围和接入状态
│   ├── model-download.mjs  # 官方模型文件下载、代理支持、磁盘检查、校验和完成文件复用
│   ├── model-download.test.mjs  # 环境识别与模型下载完整性、取消、固定来源和路径校验测试
│   ├── model-environment.mjs  # Linux 与 WSL 版本识别、GPT-SoVITS 模型完整性和本地选择读取
│   ├── model-library-asr.test.mjs  # 识别模型独立选择、持久化、依赖缺失与下载完整性测试
│   ├── model-library.mjs  # 声音生成与识别模型独立扫描选择、受管下载任务和状态汇总
│   ├── model-library.test.mjs  # 模型发现选择持久化、启动门控、外部进程保护及取消回归测试
│   ├── paths.mjs  # 启动器配置路径解析与相对项目或用户目录的可移植路径显示
│   ├── paths.test.mjs  # 项目目录、用户主目录及外部路径解析与显示回归测试
│   ├── process-cleanup.mjs  # 核对项目与进程身份后清理残留进程组并通知受管 Windows 执行端停止
│   ├── process-cleanup.test.mjs  # 残留服务正常及强制退出、孤立子进程回收和其他检出隔离测试
│   ├── server.mjs  # 受管主服务启动入口，等待 PostgreSQL 就绪后通过 Rust 缓存入口启动服务
│   ├── shutdown.mjs  # 幂等处理重复中断与终端挂断信号，等待启动器完成服务清理
│   ├── shutdown.test.mjs  # 真实进程验证重复 Ctrl+C、终止和终端挂断时等待受管服务回收
│   ├── supervisor.mjs  # 固定服务子进程的幂等启停、就绪等待、超时取消和退出回收
│   ├── supervisor.test.mjs  # 真实受控子进程的启动停止、取消、崩溃、冲突与外部服务隔离测试
│   ├── windows-client.mjs  # WSL2 调用 Windows helper、连接就绪检查及第三开关生命周期
│   ├── windows-client.test.mjs  # Windows 连接门控、重复启动、取消重试、外部保护与关闭顺序回归测试
│   └── windows-config.mjs  # 安全解析并原子更新实际 Windows 执行端 TOML，仅启用 VTS 插件连接
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── README.md  # 开发命令、Rust 构建缓存回收边界、启动器及模型训练工具说明
├── acceptance.mjs  # 只读持续观测、脱敏采样与整体验收 JSON 报告工具
├── acceptance.test.mjs  # 只读验收工具的受控 HTTP、限额、去敏和计数测试
├── asr_selection_test.py  # 训练转写读取页面模型选择、配置优先级和无效选择拒绝测试
├── directory-descriptions.json  # 可提交工程的文件和目录用途登记；索引生成的说明源
├── directory-tree.mjs  # 递归生成目录树，校验用途覆盖并检测文件内容变化
├── directory-tree.test.mjs  # 验证递归索引、文件增删改、排除规则、本地文档隔离和符号链接边界
├── engine_workspace.py  # 构建项目自有引擎副本并配置 CPU、GPU、数据加载和内存模式
├── extract-model-archive.py  # 限定路径和解压大小的 G2PW 官方模型压缩包解压器
├── model_runtime.py  # 受管 TTS 模型按需启停、合成线程隔离、并发保护及私有引擎入口适配
├── model_runtime_test.py  # 模型默认待机、加载卸载、合成期间状态响应及取消保护的无权重测试
├── rust_cache.py  # 记录成功 Cargo 构建的有效产物并在文件锁保护下回收旧可执行程序与已确认归属的增量缓存
├── rust_cache_test.py  # 用真实文件和 Cargo 验证缓存保护、失败构建、并发锁、路径边界及清理后的重复构建命中
├── start-control-panel.mjs  # 启动 Vite 控制面板、模型管理与本地服务监督器并回收受管进程
├── start-managed-inference.py  # 准备私有推理配置、默认不加载模型的受管 TTS 服务启动入口
├── stop-control-panel.mjs  # Linux 与 WSL 手动清理本项目残留服务并报告停止结果的命令入口
├── train-gpt-sovits.py  # GPT-SoVITS 分阶段训练、同音色权重续训与产物校验入口
├── training_test.py  # 验证训练路径、性能传递、阶段进程与低显存兼容行为
├── training_transcription.py  # 本地 Whisper 转写、页面模型选择与旧配置兼容、离线校验及模型卸载
├── transcribe-training.py  # 单片训练语音转写命令入口，输出供用户校对的文本
└── windows-bootstrap.test.ps1  # Windows PowerShell 入口语法、WSL 检测及参数边界回归测试
```

可继续查看各子目录的索引：

- [launcher/](launcher/DIRECTORY.md)：Linux 本机服务启动管理、配置读取、状态探测与控制接口

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: fb346b8fd3150f97ede3d2eb13a7b8bcf356a8aae5c9faa40a688ac0d42a2887 -->
