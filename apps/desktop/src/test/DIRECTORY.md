# test 目录索引

前端测试公共设施

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
test/  # 前端测试公共设施
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── agent-fixtures.ts  # Agent 面板测试的完整状态与事件夹具
├── launcher-fixtures.ts  # 三服务启动管理状态的前端测试数据
├── live-fixtures.ts  # 直播连接面板与服务客户端测试的完整快照夹具
├── model-library-fixtures.ts  # 环境检测、已安装模型与可下载模型的前端测试数据
├── resource-fixtures.ts  # 资源档案、模型、热键与音频测试样例
├── server-fixtures.ts  # 主服务响应与异步请求测试夹具
└── setup.ts  # DOM 测试匹配器与清理
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 0434ad149a500b9bd61f66968fdef3dd5525a397f4b63129102b1af9c9bb7ac0 -->
