# desktop-runtime 目录索引

独立于界面的 Windows 播放与设备执行库

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
desktop-runtime/  # 独立于界面的 Windows 播放与设备执行库
├── src/  # 播放、连接、口型、VTS、OBS 与模型导入的执行源码
└── tests/  # 桌面执行运行时独立集成测试
```

可继续查看各子目录的索引：

- [src/](src/DIRECTORY.md)：播放、连接、口型、VTS、OBS 与模型导入的执行源码
- [tests/](tests/DIRECTORY.md)：桌面执行运行时独立集成测试

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 36385af586c37cb4e3834ceb3302b36631fa1471d04e215477dab0ab2fb8d8c0 -->
