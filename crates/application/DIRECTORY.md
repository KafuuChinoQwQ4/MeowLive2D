# application 目录索引

业务用例编排及外部能力接口定义

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
application/  # 业务用例编排及外部能力接口定义
├── src/  # Agent、事件调度、语音与资源任务用例
└── tests/  # 应用用例的队列与状态流转集成测试
```

可继续查看各子目录的索引：

- [src/](src/DIRECTORY.md)：Agent、事件调度、语音与资源任务用例
- [tests/](tests/DIRECTORY.md)：应用用例的队列与状态流转集成测试

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 2c4d5daf11f17e32b01c41aba9516417c0a1dedd8277ef25fcda6539fa046f49 -->
