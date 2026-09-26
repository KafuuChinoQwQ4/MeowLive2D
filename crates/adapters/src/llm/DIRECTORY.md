# llm 目录索引

云端及本地 LLM 的协议适配

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
llm/  # 云端及本地 LLM 的协议适配
├── models/  # 模型目录配置及响应解析实现
├── reasoning/  # 具体模型推理能力登记与原生参数映射
└── runtime/  # 四协议工具调用、流式响应、缓存和用量的统一适配
```

可继续查看各子目录的索引：

- [models/](models/DIRECTORY.md)：模型目录配置及响应解析实现
- [reasoning/](reasoning/DIRECTORY.md)：具体模型推理能力登记与原生参数映射
- [runtime/](runtime/DIRECTORY.md)：四协议工具调用、流式响应、缓存和用量的统一适配

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 0418b45146b654d19db074ad1b723fb703e8bcbb6bc440bed9dfb2f66f155827 -->
