# server 目录索引

Linux / WSL 主服务入口、配置和传输边界

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
server/  # Linux / WSL 主服务入口、配置和传输边界
├── src/  # 主服务启动、配置解析及 HTTP / WebSocket 适配源码
└── tests/  # 主服务配置、HTTP、桥接与完整播报集成测试
```

可继续查看各子目录的索引：

- [src/](src/DIRECTORY.md)：主服务启动、配置解析及 HTTP / WebSocket 适配源码
- [tests/](tests/DIRECTORY.md)：主服务配置、HTTP、桥接与完整播报集成测试

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: cd7c77beb117ae9f5333e8f13b5c06c4eb111fed39f3dbdcf035d98de6f37de1 -->
