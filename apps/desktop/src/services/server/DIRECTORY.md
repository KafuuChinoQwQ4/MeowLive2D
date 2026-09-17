# server 目录索引

Rust 主服务 HTTP / WebSocket 客户端入口

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
server/  # Rust 主服务 HTTP / WebSocket 客户端入口
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── agent.bounds.test.ts  # Agent 合法最大快照、冷却与礼物数量边界回归测试
├── agent.failures.test.ts  # Agent 畸形响应、HTTP 错误、超时与取消测试
├── agent.requests.test.ts  # Agent 查询、设置、暂停恢复与事件批量请求契约测试
├── agent.ts  # 独立 Agent HTTP 客户端、超时取消与运行时响应校验
├── failures.test.ts  # HTTP 错误、协议校验、取消和超时测试
├── index.ts  # 带超时和取消的主服务 HTTP 客户端
├── live.failures.test.ts  # 直播快照边界、HTTP 错误、网络失败、超时及取消测试
├── live.requests.test.ts  # 直播连接查询、连接及断开请求契约测试
├── live.ts  # 直播连接 HTTP 客户端、超时取消与运行时快照校验
├── llm.test.ts  # LLM 客户端路由、校验、错误与超时测试
├── llm.ts  # LLM 设置与连接测试 HTTP 客户端及响应校验
├── obs.test.ts  # OBS HTTP 响应校验、失败和超时且不重放控制请求的测试
├── obs.ts  # OBS 主服务请求、超时处理和状态契约校验
├── requests.test.ts  # HTTP 请求与成功响应测试
├── resources.failures.test.ts  # 资源响应边界、请求失败、超时与取消测试
├── resources.requests.test.ts  # 资源增删接口负载、空绑定及安装模型响应关联测试
├── resources.ts  # 资源 HTTP 与桌面操作客户端及运行时响应校验
├── responses.ts  # 生成契约的运行时响应校验与错误映射
├── training.test.ts  # 训练与转写契约、请求校验、保存删除及取消超时测试
└── training.ts  # 训练性能、任务版本、模型启停与离线测量客户端及响应校验
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: e59dfa114172a8b1d8514bbbb714d1750044172cb72992527122eee8f98bcd0c -->
