# src 目录索引

播放、连接、口型、VTS、OBS 与模型导入的执行源码

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
src/  # 播放、连接、口型、VTS、OBS 与模型导入的执行源码
├── assets/  # Live2D 导出模型包的本地校验与安装
├── audio/  # 音频设备后端、采样转换与设备播放时钟
├── avatar/  # VTube Studio 私有协议、配置校验、授权存储与口型连接状态机
├── bin/  # 独立桌面执行客户端命令入口
├── lip_sync/  # 设备能量到口型的纯规则、配置与音频后端包装器
└── obs/  # OBS 本地连接配置与校验
```

可继续查看各子目录的索引：

- [assets/](assets/DIRECTORY.md)：Live2D 导出模型包的本地校验与安装
- [audio/](audio/DIRECTORY.md)：音频设备后端、采样转换与设备播放时钟
- [avatar/](avatar/DIRECTORY.md)：VTube Studio 私有协议、配置校验、授权存储与口型连接状态机
- [bin/](bin/DIRECTORY.md)：独立桌面执行客户端命令入口
- [lip_sync/](lip_sync/DIRECTORY.md)：设备能量到口型的纯规则、配置与音频后端包装器
- [obs/](obs/DIRECTORY.md)：OBS 本地连接配置与校验

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: cafdd1a21ceeccb417087ad32daf8b33b97581c0fa428bd0d9f20c9fb0190dbb -->
