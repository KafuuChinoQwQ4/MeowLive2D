# shared 目录索引

跨功能复用且不持有业务流程的前端能力

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
shared/  # 跨功能复用且不持有业务流程的前端能力
├── lib/  # 与业务状态无关的公共纯函数
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── index.ts  # 跨功能复用的纯函数；与网络、Tauri、全局业务状态无关。
├── ui/  # 纯展示公共组件
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── index.ts  # 跨功能复用的纯展示组件；不请求网络，也不导入 features。
└── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
```

可继续查看各子目录的索引：

- [lib/](lib/DIRECTORY.md)：与业务状态无关的公共纯函数
- [ui/](ui/DIRECTORY.md)：纯展示公共组件

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 4200101de900c92c6f55305d566caa9f441c613f9ab4dc0d2411259033824a32 -->
