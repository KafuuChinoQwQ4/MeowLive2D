# support 目录索引

语音适配测试的 WAV 和 HTTP 公共夹具

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
support/  # 语音适配测试的 WAV 和 HTTP 公共夹具
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── http.rs  # 受控 TTS HTTP 服务与请求配置夹具
├── mod.rs  # 内存 WAV 生成与公共夹具模块导出
└── raw_http.rs  # 分块响应测试所用原始 HTTP 请求读取夹具
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 689f3d364240796c15c980fe4371328a8115b9c7abd5b1425390a2f4ee6d2f5b -->
