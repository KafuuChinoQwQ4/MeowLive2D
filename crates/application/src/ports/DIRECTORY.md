# ports 目录索引

业务方定义的模型、语音、存储和执行能力边界

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
ports/  # 业务方定义的模型、语音、存储和执行能力边界
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── execution.rs  # Windows 执行指令下发、取消和执行回执接收的能力边界；实现不在业务层。
├── live_source.rs  # 平台无关直播源、连接生命周期和错误语义接口
├── llm.rs  # 模型决策、已完成对话与可取消异步模型接口
├── mod.rs  # 由业务方定义的外部能力接口。实现位于 adapters 或应用入口的传输适配层。
├── speech.rs  # 可动态注入的异步语音合成接口与 PCM 输出类型
├── storage.rs  # 资源快照、参考音频与引擎路径存储接口
└── training.rs  # 训练素材存储、成对模型解析与受控进程执行接口
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 2f048fae7c716a7e0878ebdf217baaca2341b4f230c11e690e941d1eba61eaa9 -->
