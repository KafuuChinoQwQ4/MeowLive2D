# scripts 目录索引

开发工具、目录用途登记与索引同步检查

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
scripts/  # 开发工具、目录用途登记与索引同步检查
├── launcher/  # Linux 本机服务启动管理、配置读取、状态探测与控制接口
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── config.mjs  # 读取私有启动配置与主服务 TOML，校验路径并仅给服务进程加载密钥
│   ├── config.test.mjs  # 启动配置、缺失环境、密钥隔离和本机地址限制的测试
│   ├── health.mjs  # 检查主服务及 TTS HTTP 就绪状态和端口占用
│   ├── http.mjs  # 同源会话保护的三个服务开关及模型管理 HTTP 接口
│   ├── http.test.mjs  # 启停接口来源、会话、请求形状及大小限制测试
│   ├── log.mjs  # 受管进程日志限量保存、密钥遮盖及常见启动故障识别
│   ├── log.test.mjs  # 子进程日志跨数据块密钥遮盖与显存错误诊断测试
│   ├── model-catalog.mjs  # 核对官方来源的公开语音模型目录、下载文件范围和接入状态
│   ├── model-download.mjs  # 官方模型文件下载、代理支持、磁盘检查、校验和完成文件复用
│   ├── model-download.test.mjs  # 环境识别与模型下载完整性、取消、固定来源和路径校验测试
│   ├── model-environment.mjs  # Linux 与 WSL 版本识别、GPT-SoVITS 模型完整性和本地选择读取
│   ├── model-library.mjs  # 本地模型扫描、已发现模型选择、受管下载任务和状态汇总
│   ├── model-library.test.mjs  # 模型发现选择持久化、启动门控、外部进程保护及取消回归测试
│   ├── paths.mjs  # 启动器配置路径解析与相对项目或用户目录的可移植路径显示
│   ├── paths.test.mjs  # 项目目录、用户主目录及外部路径解析与显示回归测试
│   ├── supervisor.mjs  # 固定服务子进程的幂等启停、就绪等待、超时取消和退出回收
│   ├── supervisor.test.mjs  # 真实受控子进程的启动停止、取消、崩溃、冲突与外部服务隔离测试
│   ├── windows-client.mjs  # WSL2 调用 Windows helper、连接就绪检查及第三开关生命周期
│   └── windows-client.test.mjs  # Windows 连接门控、重复启动、取消重试、外部保护与关闭顺序回归测试
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── README.md  # 开发验证命令、网页启动管理及引擎工具说明
├── acceptance.mjs  # 只读持续观测、脱敏采样与整体验收 JSON 报告工具
├── acceptance.test.mjs  # 只读验收工具的受控 HTTP、限额、去敏和计数测试
├── directory-descriptions.json  # 可提交工程的文件和目录用途登记；索引生成的说明源
├── directory-tree.mjs  # 递归生成目录树，校验用途覆盖并检测文件内容变化
├── directory-tree.test.mjs  # 验证递归索引、文件增删改、排除规则、本地文档隔离和符号链接边界
├── engine_workspace.py  # 独立引擎镜像与选定模型资源隔离，训练及推理的离线环境配置
├── extract-model-archive.py  # 限定路径和解压大小的 G2PW 官方模型压缩包解压器
├── start-control-panel.mjs  # 启动 Vite 控制面板、模型管理与本地服务监督器并回收受管进程
├── start-managed-inference.py  # 按选定模型目录和私有配置启动 GPT-SoVITS v2 推理服务
├── train-gpt-sovits.py  # 逐阶段 GPT-SoVITS v2 预处理训练及最终成对模型校验桥接
├── training_test.py  # 训练清单、受限环境、低显存配置及模型压缩包解压边界回归测试
└── windows-bootstrap.test.ps1  # Windows PowerShell 入口语法、WSL 检测及参数边界回归测试
```

可继续查看各子目录的索引：

- [launcher/](launcher/DIRECTORY.md)：Linux 本机服务启动管理、配置读取、状态探测与控制接口

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 46351f169ff711fb4030144950cd2f3302f256549fd172bc6c8621c9abb94ad8 -->
