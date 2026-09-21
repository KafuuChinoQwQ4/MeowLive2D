# audio 目录索引

音频设备后端、采样转换与设备播放时钟

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
audio/  # 音频设备后端、采样转换与设备播放时钟
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── buffer.rs  # 跨平台重采样输出缓冲，按需分配并限制实际排队 PCM 为 128 MiB，附长 SC 和溢出测试
├── conversion.rs  # 相位连续的 PCM 重采样与声道映射
├── meter.rs  # 固定容量的设备播放能量时间线与 RMS 累加器
├── simulated.rs  # 显式静音模拟后端，按模拟播放时间退役样本并观测能量
├── timing.rs  # 设备计划播放时间与完成回执时钟
└── windows.rs  # Windows 默认输出设备、受限按需重采样缓冲及真实输出能量和时钟回执
```

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: d98f21c26f921a0dfd93335d7a61f304c0abeb7ba21a1779c652b74d7f6e05cf -->
