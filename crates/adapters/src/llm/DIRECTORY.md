# llm 目录索引

云端及本地 LLM 的协议适配

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
llm/  # 云端及本地 LLM 的协议适配
├── models/  # 模型目录配置及响应解析实现
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── config.rs  # 模型目录地址规范化与密钥校验
│   └── response.rs  # 供应商模型条目及分页元数据解析
├── reasoning/  # 具体模型推理能力登记与原生参数映射
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── capabilities.rs  # 经官方文档核对的模型推理档位和预算预设能力表
├── runtime/  # 四协议工具调用、流式响应、缓存和用量的统一适配
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── mod.rs  # 有界单轮模型调用与只读流式观测
│   ├── output.rs  # 原生输出、工具签名与用量归一化
│   ├── request.rs  # 原生工具、续接上下文与稳定缓存前缀请求
│   └── stream.rs  # 四协议 SSE 拼包、结束验证与用量保留
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── config.rs  # LLM 地址规范化、认证头及资源限制的统一校验
├── mod.rs  # 模型协议适配。兼容同一协议的云端与本地服务复用实现，其他协议独立添加。
├── models.rs  # 受限的供应商模型目录 HTTP 适配与分页汇总
├── multi_provider.rs  # 多提供商决策与原生工具、流式运行层适配
├── openai_compatible.rs  # 非流式 Chat Completions 传输、认证、响应大小与临时错误分类
├── prompt.rs  # 跨 LLM 协议共享的事件回复与主动发言提示隔离及输入校验
├── reasoning.rs  # 推理档位夹取、预算限制和各协议原生请求参数注入
└── response.rs  # LLM 文本决策内容与 OpenAI 聊天响应的严格校验
```

可继续查看各子目录的索引：

- [models/](models/DIRECTORY.md)：模型目录配置及响应解析实现
- [reasoning/](reasoning/DIRECTORY.md)：具体模型推理能力登记与原生参数映射
- [runtime/](runtime/DIRECTORY.md)：四协议工具调用、流式响应、缓存和用量的统一适配

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: e211b02b16f84093348a309e23d1049f92edc711f2d29048238e1392353ab1b6 -->
