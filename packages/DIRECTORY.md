# packages 目录索引

TypeScript 共享工作区包

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
packages/  # TypeScript 共享工作区包
├── contracts/  # 供前端使用的跨端协议类型出口
│   ├── src/  # 从 Rust 生成的 TypeScript 协议类型和版本常量
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   └── index.ts  # 自动生成的前端跨端协议类型、Agent 观察模型与版本常量
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── README.md  # 协议类型来源、生成命令和消费方式
│   ├── package.json  # 前端协议类型包的导出边界与类型检查命令
│   └── tsconfig.json  # 共享协议类型包的 TypeScript 检查范围
└── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
```

可继续查看各子目录的索引：

- [contracts/](contracts/DIRECTORY.md)：供前端使用的跨端协议类型出口

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 48eff744e443154e6bfcc6c121246d44ab9caf5823262a41ef9d71024c43fbb9 -->
