# training 目录索引

独立训练进程、进度及产物的适配

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
training/  # 独立训练进程、进度及产物的适配
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── mod.rs  # 独立训练进程适配，处理进程状态与训练产物；调度策略属于 application。
├── process.rs  # 独立训练进程组启动、超时取消和有界日志进度
├── store.rs  # 训练任务原子快照、保存音色兼容加载、不可变素材及成对权重指纹校验
└── tests.rs  # 任务与音色保存持久化、旧存档兼容、存储故障、模型校验和真实子进程清理测试
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: b48e5828b4b07ab27b423b0af4c548f66082c569925e7c91ce33101b83297fe1 -->
