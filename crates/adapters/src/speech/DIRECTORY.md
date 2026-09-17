# speech 目录索引

GPT-SoVITS 等语音引擎的请求和音频格式适配

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
speech/  # GPT-SoVITS 等语音引擎的请求和音频格式适配
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── gpt_sovits.rs  # GPT-SoVITS HTTP 适配入口：参考素材路径解析、合成参数映射与音频解码。
├── mod.rs  # 语音引擎适配与音频格式转换。推理服务和模型权重位于项目目录之外。
├── model_synthesizer.rs  # 模型内存启停、成对权重加载与合成互斥及取消事务保护
├── resource_synthesizer.rs  # 根据音色档案解析参考音频并调用 GPT-SoVITS
└── wav.rs  # 完整 RIFF/WAV 边界与 PCM16 格式校验解码
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 31d2d3f8350cca1f97a8877ff4f1962087fd486f8f74679762d637222a2a6687 -->
