# services 目录索引

前端访问主服务、Linux 启动管理与 Windows 桌面能力的统一边界

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
services/  # 前端访问主服务、Linux 启动管理与 Windows 桌面能力的统一边界
├── audio/  # 参考音频与训练片段的浏览器解码、格式转换和音频校验
├── desktop/  # Tauri 命令客户端与桌面能力边界
├── launcher/  # 前端访问本机启动管理器的服务适配层
├── model-library/  # 前端访问启动管理器语音模型能力的服务适配层
└── server/  # Rust 主服务 HTTP / WebSocket 客户端入口
```

可继续查看各子目录的索引：

- [audio/](audio/DIRECTORY.md)：参考音频与训练片段的浏览器解码、格式转换和音频校验
- [desktop/](desktop/DIRECTORY.md)：Tauri 命令客户端与桌面能力边界
- [launcher/](launcher/DIRECTORY.md)：前端访问本机启动管理器的服务适配层
- [model-library/](model-library/DIRECTORY.md)：前端访问启动管理器语音模型能力的服务适配层
- [server/](server/DIRECTORY.md)：Rust 主服务 HTTP / WebSocket 客户端入口

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 89beec275991279445c52beb8039a97e9b142d47c00c0d55c6893b78601a39d6 -->
