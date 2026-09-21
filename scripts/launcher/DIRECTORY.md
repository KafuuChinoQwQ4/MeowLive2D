# launcher 目录索引

Linux 本机服务启动管理、配置读取、状态探测与控制接口

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
launcher/  # Linux 本机服务启动管理、配置读取、状态探测与控制接口
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── config.mjs  # 私有启动配置、默认观众存储元数据及主服务环境的加载与脱敏
├── config.test.mjs  # 启动配置、缺失环境、密钥隔离和本机地址限制的测试
├── database.mjs  # 默认观众存储的私有数据库凭据准备、项目 PostgreSQL 启动和受限应用连接注入
├── database.test.mjs  # 默认数据库首次准备、凭据复用、外部连接隔离和错误脱敏测试
├── health.mjs  # 通过公开最小健康接口检查主服务与 TTS 就绪和端口占用
├── health.test.mjs  # 认证主服务通过公开最小健康接口被启动器和设备探测的测试
├── http.mjs  # 同源会话保护的三个服务开关及模型管理 HTTP 接口
├── http.test.mjs  # 启停接口来源、会话、请求形状及大小限制测试
├── log.mjs  # 受管进程日志限量保存、密钥遮盖及常见启动故障识别
├── log.test.mjs  # 子进程日志跨数据块密钥遮盖与显存错误诊断测试
├── memory.mjs  # 读取 Linux/WSL 与 Windows 主机内存余量，提供受管 TTS 启动和运行保护判定
├── model-asr.mjs  # 识别模型选择读取、Whisper 文件完整性和训练 Python 依赖检查
├── model-catalog-asr.mjs  # Whisper Turbo 与其他可选本地语音识别模型的来源、文件范围和适配状态
├── model-catalog.mjs  # 核对官方来源的公开语音模型目录、下载文件范围和接入状态
├── model-download.mjs  # 官方模型文件下载、代理支持、磁盘检查、校验和完成文件复用
├── model-download.test.mjs  # 环境识别与模型下载完整性、取消、固定来源和路径校验测试
├── model-environment.mjs  # Linux 与 WSL 版本识别、GPT-SoVITS 模型完整性和本地选择读取
├── model-library-asr.test.mjs  # 识别模型独立选择、持久化、依赖缺失与下载完整性测试
├── model-library.mjs  # 声音生成与识别模型独立扫描选择、受管下载任务和状态汇总
├── model-library.test.mjs  # 模型发现选择持久化、启动门控、外部进程保护及取消回归测试
├── paths.mjs  # 启动器配置路径解析与相对项目或用户目录的可移植路径显示
├── paths.test.mjs  # 项目目录、用户主目录及外部路径解析与显示回归测试
├── process-cleanup.mjs  # 核对项目与进程身份后清理残留进程组并通知受管 Windows 执行端停止
├── process-cleanup.test.mjs  # 残留服务正常及强制退出、孤立子进程回收和其他检出隔离测试
├── server.mjs  # 受管主服务启动入口，等待 PostgreSQL 就绪后通过 Rust 缓存入口启动服务
├── shutdown.mjs  # 幂等处理重复中断与终端挂断信号，等待启动器完成服务清理
├── shutdown.test.mjs  # 真实进程验证重复 Ctrl+C、终止和终端挂断时等待受管服务回收
├── supervisor.mjs  # 固定服务子进程的幂等启停、就绪等待、超时取消和退出回收
├── supervisor.test.mjs  # 真实受控子进程的启动停止、取消、崩溃、冲突与外部服务隔离测试
├── windows-client.mjs  # WSL2 调用 Windows helper、连接就绪检查及第三开关生命周期
├── windows-client.test.mjs  # Windows 连接门控、重复启动、取消重试、外部保护与关闭顺序回归测试
└── windows-config.mjs  # 安全解析并原子更新实际 Windows 执行端 TOML，仅启用 VTS 插件连接
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 97923df94934d469e5146aa951449b6ba64691d6e2b5847c276f77e9e550147d -->
