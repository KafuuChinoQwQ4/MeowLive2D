# bilibili 目录索引

哔哩哔哩官方直播开放平台的授权、签名、连接与事件适配

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
bilibili/  # 哔哩哔哩官方直播开放平台的授权、签名、连接与事件适配
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── config.rs  # 哔哩哔哩接入凭据、网络地址与资源上限配置校验
├── connection.rs  # 官方直播长连接鉴权、接收、心跳与显式会话清理
├── http.rs  # 开放平台签名 HTTP 项目开始、心跳、结束与错误处理
├── mod.rs  # 哔哩哔哩直播适配器的模块与配置公开出口
├── protocol.rs  # 官方直播二进制包解析、有界解压、鉴权回复与领域事件转换
└── signing.rs  # 开放平台请求体 MD5 与规范请求头 HMAC-SHA256 签名
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 04fa9f018d08e0fb309af7cc5d63680bafc77847b2f690b4e633187a6a83c22d -->
