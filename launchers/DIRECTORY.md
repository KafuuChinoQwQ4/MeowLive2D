# launchers 目录索引

面向用户的 Windows 双击与 Linux 启动入口及 Windows 进程管理脚本

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
launchers/  # 面向用户的 Windows 双击与 Linux 启动入口及 Windows 进程管理脚本
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── README.md  # 首次环境准备、Windows 执行端构建与日常启动退出说明
├── start-windows.cmd  # Windows 双击启动入口，调用 WSL2 检查与控制面板启动脚本
├── start-windows.ps1  # 检测 WSL2、选择发行版、启动 Linux 项目并打开浏览器
├── start.sh  # Linux 与 WSL 新人控制面板入口，检查依赖并启动网页服务管理器
└── windows-client.ps1  # 绑定 Job Object 和租约的 Windows 执行端后台启动与回收
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: dac8653fde62069ffc37b7324b1d8e1f71e5018f809ee18595064d9c12ed8ac4 -->
