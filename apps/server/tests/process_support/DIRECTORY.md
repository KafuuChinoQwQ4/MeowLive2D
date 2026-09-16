# process_support 目录索引

真实主服务进程测试的隔离目录和音频构造辅助

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
process_support/  # 真实主服务进程测试的隔离目录和音频构造辅助
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
└── mod.rs  # 主服务测试子进程启动清理和受控 WAV 构造
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: aebeda7854f3018fa68e3d31698935dcca3a837355a24fa3bc17c3287654a069 -->
