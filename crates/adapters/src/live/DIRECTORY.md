# live 目录索引

直播源连接、事件标准化与模拟输入

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
live/  # 直播源连接、事件标准化与模拟输入
└── bilibili/  # 哔哩哔哩官方直播开放平台的授权、签名、连接与事件适配
```

可继续查看各子目录的索引：

- [bilibili/](bilibili/DIRECTORY.md)：哔哩哔哩官方直播开放平台的授权、签名、连接与事件适配

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 2e4653a24e3f21c02eaee0ae37c3794c65d5e7b7565c0ec0d84550e4fc540523 -->
