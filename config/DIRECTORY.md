# config 目录索引

可提交的 Linux 与 Windows 配置示例

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
config/  # 可提交的 Linux 与 Windows 配置示例
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── README.md  # 主服务、启动管理和 Windows 执行端的配置边界与启用说明
├── desktop.example.toml  # Windows 执行端连接、缓冲、VTS 插件与口型的默认关闭示例配置
├── launcher.example.json  # 网页启动器的主服务配置、TTS 环境与本地密钥文件路径模板
├── offline.example.toml  # 本地兼容量化 LLM 与 TTS 的受限离线运行配置样例
└── server.example.toml  # 主服务、GPT-SoVITS、默认暂停 Agent、LLM 和默认禁用直播接入的示例参数
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: afc8921d23a4cf3c77d667936c6bbcf324af9ebeaf5edab9665097403ee06077 -->
