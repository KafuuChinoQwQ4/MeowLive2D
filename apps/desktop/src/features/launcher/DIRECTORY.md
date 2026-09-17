# launcher 目录索引

控制面板服务开关、启动状态及新人引导

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
launcher/  # 控制面板服务开关、启动状态及新人引导
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── LauncherControls.tsx  # 主服务、TTS 和 Windows 执行端三开关、新人步骤与配置帮助
├── LauncherPanel.test.tsx  # 滑动开关真实状态、启动取消、外部服务与断线交互测试
├── LauncherPanel.tsx  # 服务开关展示与业务就绪门控的可复用组合入口
├── index.ts  # 服务开关展示、状态控制器与组合面板公共导出
└── useLauncher.ts  # 启动管理状态轮询和串行启停请求，防止过期状态覆盖
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: e7ef4f9e6d0d4451e9d83dd1d90809c8a04de9ae559e454cc346a0018e66ed5b -->
