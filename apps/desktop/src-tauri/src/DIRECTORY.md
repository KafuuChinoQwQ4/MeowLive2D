# src 目录索引

桌面启动、依赖组装与命令转发源码

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
src/  # 桌面启动、依赖组装与命令转发源码
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── bootstrap.rs  # Windows Tauri 窗口组装、宿主启动及退出取消清理
├── commands.rs  # 仅暴露配置地址与执行宿主状态的 Tauri IPC 命令
├── lib.rs  # Windows 桌面外壳库入口，执行逻辑委托独立运行库
├── main.rs  # 启动桌面外壳并报告启动失败的进程入口
└── startup.rs  # 桌面启动参数、首次配置创建及相对路径解析
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 894a8415e16a82d2576e08ec376583bd3a023815b57afbc38704472d2d7854cc -->
