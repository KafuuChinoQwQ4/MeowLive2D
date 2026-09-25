# MeowLive2D 目录索引

MeowLive2D：Rust 主服务、Windows 执行层与 TypeScript 控制面板

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
MeowLive2D/  # MeowLive2D：Rust 主服务、Windows 执行层与 TypeScript 控制面板
├── apps/  # 可执行应用入口与依赖组装
├── config/  # 独立数据库部署配置与 Linux、Windows 配置示例
├── crates/  # 按职责与单向依赖隔离的 Rust 库
├── docs/  # 本地架构与规划文档；完整树见 docs/DIRECTORY.md，Git 忽略且可不存在
├── launchers/  # 面向用户的 Windows 双击与 Linux 启动入口及 Windows 进程管理脚本
├── packages/  # TypeScript 共享工作区包
├── scripts/  # 开发工具、目录用途登记与索引同步检查
└── tests/  # 业务测试归属说明与回放数据入口
```

可继续查看各子目录的索引：

- [apps/](apps/DIRECTORY.md)：可执行应用入口与依赖组装
- [config/](config/DIRECTORY.md)：独立数据库部署配置与 Linux、Windows 配置示例
- [crates/](crates/DIRECTORY.md)：按职责与单向依赖隔离的 Rust 库
- [docs/](docs/DIRECTORY.md)：本地架构与规划文档；完整树见 docs/DIRECTORY.md，Git 忽略且可不存在
- [launchers/](launchers/DIRECTORY.md)：面向用户的 Windows 双击与 Linux 启动入口及 Windows 进程管理脚本
- [packages/](packages/DIRECTORY.md)：TypeScript 共享工作区包
- [scripts/](scripts/DIRECTORY.md)：开发工具、目录用途登记与索引同步检查
- [tests/](tests/DIRECTORY.md)：业务测试归属说明与回放数据入口

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: e4fbed0aff1b87dc04cd57ab32eaa1a12564c9add29f8e9fbabd18f713507635 -->
