# src-tauri 目录索引

Windows Tauri 薄外壳、窗口配置与执行库生命周期

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
src-tauri/  # Windows Tauri 薄外壳、窗口配置与执行库生命周期
├── capabilities/  # 桌面窗口的 Tauri 能力声明
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── default.json  # 主窗口的最小权限范围，未开放系统插件能力
├── icons/  # 沿用控制台绿色 M 标志的图标源及窗口构建资源
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── icon.ico  # Windows 窗口与应用资源使用的 ICO 图标
│   ├── icon.png  # Tauri 使用的 RGBA 窗口图标
│   └── icon.svg  # 绿色 M 应用图标的可编辑矢量源
├── src/  # 桌面启动、依赖组装与命令转发源码
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── bootstrap.rs  # Windows Tauri 窗口组装、宿主启动及退出取消清理
│   ├── commands.rs  # 仅暴露配置地址与执行宿主状态的 Tauri IPC 命令
│   ├── lib.rs  # Windows 桌面外壳库入口，执行逻辑委托独立运行库
│   ├── main.rs  # 启动桌面外壳并报告启动失败的进程入口
│   └── startup.rs  # 桌面启动参数、首次配置创建及相对路径解析
├── tests/  # 可跨平台验证的桌面配置与启动测试
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── configuration.rs  # 首次配置、保留用户设置、参数和相对路径解析测试
├── Cargo.toml  # 该 Rust 包的名称、workspace 配置与模块依赖声明
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── README.md  # 桌面入口状态、Tauri 接入边界及 Windows 验证要求
├── build.rs  # Windows Tauri 构建资源与权限生成入口
└── tauri.conf.json  # Tauri 窗口、构建地址、CSP 与后续安装包配置
```

可继续查看各子目录的索引：

- [capabilities/](capabilities/DIRECTORY.md)：桌面窗口的 Tauri 能力声明
- [icons/](icons/DIRECTORY.md)：沿用控制台绿色 M 标志的图标源及窗口构建资源
- [src/](src/DIRECTORY.md)：桌面启动、依赖组装与命令转发源码
- [tests/](tests/DIRECTORY.md)：可跨平台验证的桌面配置与启动测试

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 5749c1d563c5c0dbfd5b4a16d32c5006da10a319f22e17c27063b3f8bb8e01f7 -->
