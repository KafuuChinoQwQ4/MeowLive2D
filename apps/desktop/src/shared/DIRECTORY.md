# shared 目录索引

跨功能复用且不持有业务流程的前端能力

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
shared/  # 跨功能复用且不持有业务流程的前端能力
├── lib/  # 与业务状态无关的公共纯函数
└── ui/  # 纯展示公共组件
```

可继续查看各子目录的索引：

- [lib/](lib/DIRECTORY.md)：与业务状态无关的公共纯函数
- [ui/](ui/DIRECTORY.md)：纯展示公共组件

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 56619fc03a45fe48f62698c1e617809febca2774d073a6ee79a5f5d912fceeb9 -->
