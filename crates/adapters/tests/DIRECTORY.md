# tests 目录索引

GPT-SoVITS、WAV 与 LLM 适配器的集成和输入输出边界测试

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
tests/  # GPT-SoVITS、WAV 与 LLM 适配器的集成和输入输出边界测试
├── bilibili_support/  # 受控官方直播 HTTP 和 WebSocket 协议测试服务
├── llm_support/  # LLM 适配器测试公共夹具
└── support/  # 语音适配测试的 WAV 和 HTTP 公共夹具
```

可继续查看各子目录的索引：

- [bilibili_support/](bilibili_support/DIRECTORY.md)：受控官方直播 HTTP 和 WebSocket 协议测试服务
- [llm_support/](llm_support/DIRECTORY.md)：LLM 适配器测试公共夹具
- [support/](support/DIRECTORY.md)：语音适配测试的 WAV 和 HTTP 公共夹具

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 58719a50aae7ba862f7674cac5b5f95c7ac5710162dc8fb088f157d9dc4f2b1e -->
