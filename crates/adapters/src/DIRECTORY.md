# src 目录索引

按外部能力组织的适配器源码

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
src/  # 按外部能力组织的适配器源码
├── live/  # 直播源连接、事件标准化与模拟输入
├── llm/  # 云端及本地 LLM 的协议适配
├── speech/  # GPT-SoVITS 等语音引擎的请求和音频格式适配
├── storage/  # PostgreSQL 观众事件与 Linux 素材文件存储
└── training/  # 独立训练进程、进度及产物的适配
```

可继续查看各子目录的索引：

- [live/](live/DIRECTORY.md)：直播源连接、事件标准化与模拟输入
- [llm/](llm/DIRECTORY.md)：云端及本地 LLM 的协议适配
- [speech/](speech/DIRECTORY.md)：GPT-SoVITS 等语音引擎的请求和音频格式适配
- [storage/](storage/DIRECTORY.md)：PostgreSQL 观众事件与 Linux 素材文件存储
- [training/](training/DIRECTORY.md)：独立训练进程、进度及产物的适配

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 354045f37360c0b811aae66972c78f4da2b3eff6a482220b9ac2b01366a558ad -->
