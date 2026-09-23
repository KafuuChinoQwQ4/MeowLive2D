# voices 目录索引

参考素材、音色试听和训练任务界面

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
voices/  # 参考素材、音色试听和训练任务界面
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── VoicePanel.test.tsx  # 多格式音色上传选择试听、删除确认失败及空状态交互测试
├── VoicePanel.transcription.test.tsx  # 参考音频自动回填、同源音频文本上传及转写竞态回归测试
├── VoicePanel.tsx  # 音色导入校验、参考文本自动转写、选择试听和删除管理及训练入口
├── index.ts  # 音色管理：参考素材、试听与训练任务展示；不在浏览器执行模型推理。
├── types.ts  # 音色列表选择试听、上传删除及文件清理重试能力契约
└── useReferenceTranscription.ts  # 参考音频本地自动转写、手工文本保护及过期请求取消
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 486e991282f059952aafe87fa753d247f5d29564e25d35c3bc258f4fbaad413f -->
