# launcher 目录索引

前端访问本机启动管理器的服务适配层

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
launcher/  # 前端访问本机启动管理器的服务适配层
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── index.test.ts  # 启动客户端状态校验、操作拒绝、超时和不重试测试
└── index.ts  # 严格校验三服务状态并访问带令牌的本机启停接口
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 5f6226b0846d02bcc3cccea3c2afbccbfdfb793e7d084c556f32dd8ef1731763 -->
