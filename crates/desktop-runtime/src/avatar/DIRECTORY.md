# avatar 目录索引

VTube Studio 私有协议、配置校验、授权存储与口型连接状态机

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
avatar/  # VTube Studio 私有协议、配置校验、授权存储与口型连接状态机
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── client.rs  # 有界 VTS WebSocket 请求响应、请求关联、API 错误和口型参数注入
├── config.rs  # VTS 默认关闭配置与地址、参数名称、超时范围校验
├── resources.rs  # VTS 已授权模型查询加载与热键核对预览
├── switching.rs  # 切换角色口型输入并等待 VTS 应用确认
├── token.rs  # 阻塞线程池中的私有授权令牌读写、权限校验及可取消等待
└── worker.rs  # 最新口型值消费、授权复用、归零、样本过期与自动重连状态机
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 5778f683be4ec87725531a231ae2d01ddf55596149b43924a9c53dabb87158e8 -->
