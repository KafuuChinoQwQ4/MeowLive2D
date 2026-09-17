# audio 目录索引

参考音频与训练片段的浏览器解码、格式转换和音频校验

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
audio/  # 参考音频与训练片段的浏览器解码、格式转换和音频校验
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── index.test.ts  # 多格式音频转换、PCM 编码、文件大小、时长与解码失败测试
├── index.ts  # MP3 等多格式音频导入、离线解码与兼容 PCM16 WAV 转换
├── wav.test.ts  # 参考与训练音频的 WAV 格式、时长、容量与静音边界测试
└── wav.ts  # 参考音频与训练片段的 PCM16 WAV 结构及有效性校验
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: fbdc2b50ed959a8f7215facabb7816ac6f77f62760ecebba6c68676bcdc149a0 -->
