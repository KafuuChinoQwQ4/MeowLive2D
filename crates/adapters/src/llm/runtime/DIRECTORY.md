# runtime 目录索引

四协议工具调用、流式响应、缓存和用量的统一适配

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
runtime/  # 四协议工具调用、流式响应、缓存和用量的统一适配
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── mod.rs  # 有界单轮模型调用与只读流式观测
├── output.rs  # 原生输出、工具签名与用量归一化
├── request.rs  # 原生工具、续接上下文与稳定缓存前缀请求
└── stream.rs  # 四协议 SSE 拼包、结束验证与用量保留
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 4370c7653bf09f4ab21b6c65d73510090d55c1dc0a4d9f2d63100fff1aea03dc -->
