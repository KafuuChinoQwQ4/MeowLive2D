# voices 目录索引

参考素材、音色试听和训练任务界面

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
voices/  # 参考素材、音色试听和训练任务界面
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── VoicePanel.test.tsx  # 音色上传、选择、试听与缺失资源交互测试
├── VoicePanel.tsx  # 音色列表、参考音频上传校验、试听及已保存训练音色入口
├── index.ts  # 音色管理：参考素材、试听与训练任务展示；不在浏览器执行模型推理。
├── types.ts  # 音色面板需要的状态与操作接口
├── wav.test.ts  # 参考音频格式、时长、容量与静音边界测试
└── wav.ts  # 浏览器端参考 PCM16 WAV 结构与有效性校验
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 59ed3853509e88f5cba0bb498a8e3266d362d745ab49f5c1f467a6c9442df2e8 -->
