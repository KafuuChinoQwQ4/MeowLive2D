# training 目录索引

独立训练进程、进度及产物的适配

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
training/  # 独立训练进程、进度及产物的适配
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── mod.rs  # 独立训练进程适配，处理进程状态与训练产物；调度策略属于 application。
├── process.rs  # 独立训练进程组启动、超时取消和有界日志进度
├── store.rs  # 任务素材与续训权重私有副本、原子历史存储和成对模型校验
├── tests.rs  # 训练音色持久化、音频模式、转写进程、删除清理及存储故障测试
└── transcription.rs  # 单片音频自动转写的有界子进程、私有临时素材与退出清理
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: faa049822829a85f0efcd586c59269a666c3408ce5ccd33d60ad03c28ed95565 -->
