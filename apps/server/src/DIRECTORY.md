# src 目录索引

主服务启动、配置解析及 HTTP / WebSocket 适配源码

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
src/  # 主服务启动、配置解析及 HTTP / WebSocket 适配源码
├── agent/  # Agent 服务状态、跨端映射和异步模型调度
├── agent_observability/  # Agent Trace 内存状态机、持久恢复与有界保留实现
├── config/  # 按 Agent 与模型能力拆分的配置校验
├── live/  # 官方直播源组装及异步连接、接收、清理与重连驱动
├── llm_runtime/  # 运行配置、计量与持久化实现
└── transport/  # 控制接口、跨端连接与协议到领域对象的转换
```

可继续查看各子目录的索引：

- [agent/](agent/DIRECTORY.md)：Agent 服务状态、跨端映射和异步模型调度
- [agent_observability/](agent_observability/DIRECTORY.md)：Agent Trace 内存状态机、持久恢复与有界保留实现
- [config/](config/DIRECTORY.md)：按 Agent 与模型能力拆分的配置校验
- [live/](live/DIRECTORY.md)：官方直播源组装及异步连接、接收、清理与重连驱动
- [llm_runtime/](llm_runtime/DIRECTORY.md)：运行配置、计量与持久化实现
- [transport/](transport/DIRECTORY.md)：控制接口、跨端连接与协议到领域对象的转换

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 383500026de42567cadbb216589cc5addcae98770d6ed1a981476bf4acf5e679 -->
