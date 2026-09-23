# server 目录索引

Rust 主服务 HTTP / WebSocket 客户端入口

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
server/  # Rust 主服务 HTTP / WebSocket 客户端入口
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── agent-observability.test.ts  # Agent 观察查询 URL、严格响应、安全来源、错误和取消测试
├── agent-observability.ts  # 带认证和严格运行时校验的 Agent 调度及 Trace 查询客户端
├── agent.bounds.test.ts  # Agent 合法最大快照、冷却与礼物数量边界回归测试
├── agent.failures.test.ts  # Agent 畸形响应、HTTP 错误、超时与取消测试
├── agent.interaction.test.ts  # Agent 互动配置及 SC、进房事件响应校验测试
├── agent.requests.test.ts  # Agent 查询、设置、暂停恢复与事件批量请求契约测试
├── agent.ts  # 独立 Agent HTTP 客户端、超时取消与运行时响应校验
├── auth.test.ts  # 会话登录撤销、来源隔离及并发请求回归测试
├── auth.ts  # 按主服务来源保存在内存的管理员会话与认证请求封装
├── companionship.test.ts  # 陪伴管理服务契约与无效响应测试
├── companionship.ts  # 陪伴详情、积分调整撤销和礼物确认请求
├── eventPayload.ts  # 聊天、礼物、SC 与进房事件载荷共用校验
├── failures.test.ts  # HTTP 错误、协议校验、取消和超时测试
├── index.ts  # 带超时和取消的主服务 HTTP 客户端
├── live.failures.test.ts  # 直播请求异常、超时、取消及脱敏配置快照校验测试
├── live.requests.test.ts  # 直播控制和凭据配置请求、方法与载荷测试
├── live.ts  # 直播状态与控制、面板凭据设置读写及响应校验客户端
├── llm-runtime.test.ts  # 运行服务路由、契约、错误脱敏和超时取消测试
├── llm-runtime.ts  # 运行设置、可关联 Trace 的用量和活动 HTTP 客户端及严格响应验证
├── llm.reasoning.test.ts  # 推理预览 HTTP 契约、档位和预算响应校验测试
├── llm.test.ts  # LLM 配置、模型目录及连接测试客户端的契约与异常测试
├── llm.ts  # LLM 配置、模型列表与推理预览的认证请求、契约校验及取消处理
├── memory.test.ts  # 记忆管理请求、版本和响应边界测试
├── memory.ts  # 记忆查询及有条件管理操作服务
├── obs.test.ts  # OBS 控制和设置请求、地址响应校验、失败超时及不重放请求测试
├── obs.ts  # OBS 控制和脱敏连接设置读写的有界 HTTP 客户端
├── relationships.test.ts  # 关系服务请求参数和响应边界测试
├── relationships.ts  # 关系事实操作及图任务状态重建服务
├── requests.test.ts  # HTTP 请求与成功响应测试
├── resources.failures.test.ts  # 资源响应边界、请求失败、超时与取消测试
├── resources.requests.test.ts  # 资源增删接口负载、空绑定及安装模型响应关联测试
├── resources.ts  # 资源 HTTP 与桌面操作客户端及运行时响应校验
├── responses.ts  # 生成契约的运行时响应校验与错误映射
├── training.test.ts  # 训练与转写契约、请求校验、保存删除及取消超时测试
├── training.ts  # 训练性能、任务版本、模型启停与离线测量客户端及响应校验
├── viewerMerge.test.ts  # 合并预览和应用请求契约测试
├── viewerMerge.ts  # 身份合并预览与确认服务请求
├── viewers.test.ts  # 观众查询地址、分页边界与服务错误测试
└── viewers.ts  # 管理员观众与持久事件分页 HTTP 客户端
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 20b7fc22eca945c20c41f2b24eeeb97c9ed990a6deaf378251173ba6d2ba5a40 -->
