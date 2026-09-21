# ports 目录索引

业务方定义的模型、语音、存储和执行能力边界

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
ports/  # 业务方定义的模型、语音、存储和执行能力边界
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── companionship.rs  # 陪伴账本礼物确认与完成回执存储接口
├── execution.rs  # Windows 执行指令下发、取消和执行回执接收的能力边界；实现不在业务层。
├── live_source.rs  # 平台无关直播源、连接生命周期和错误语义接口
├── llm.rs  # 模型决策、已完成对话与可取消异步模型接口
├── llm_runtime.rs  # 统一模型工具轮次、流式观测与 token 用量接口
├── memory.rs  # 记忆提取与向量嵌入能力接口
├── memory_store.rs  # 权威记忆、后台任务及管理恢复存储接口
├── mod.rs  # 由业务方定义的外部能力接口。实现位于 adapters 或应用入口的传输适配层。
├── reasoning.rs  # 厂商无关的八档推理强度顺序、默认值与严格解析
├── receipt_journal.rs  # 播放完成回执本地持久暂存与数据库提交确认接口
├── relationships.rs  # 关系事实、图投影和同步恢复能力接口
├── speech.rs  # 可动态注入的异步语音合成接口与 PCM 输出类型
├── storage.rs  # 资源快照、参考音频与引擎路径存储接口
├── training.rs  # 训练存储、同音色续训基底与受控进程接口
├── viewer_merge.rs  # 身份合并预览及版本条件应用接口
├── viewers.rs  # 观众身份和直播事件的幂等持久接收及分页查询端口
└── web_search.rs  # 只读网页搜索结果与异步查询业务接口
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: b219c43e8add160424c81538ac21632f16985d8618b4364ece2d62060d84da11 -->
