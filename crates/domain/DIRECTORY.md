# domain 目录索引

业务对象、状态和不变量，保持无外部依赖

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
domain/  # 业务对象、状态和不变量，保持无外部依赖
├── src/  # 事件、会话、角色、音色和执行状态的领域定义
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── character.rs  # 角色能力映射、验证状态与口型参数规则
│   ├── event.rs  # 统一直播事件及输入长度、身份、礼物数量校验
│   ├── lib.rs  # 业务领域：只表达业务对象、状态和不变量，不依赖通信、框架或外部服务。
│   ├── performance.rs  # 发言与角色动作的执行意图和状态；播放回执决定实际执行结果。
│   ├── resources.rs  # 音色、参考素材、资源目录及纯业务不变量
│   ├── session.rs  # 直播会话的状态与合法状态转换；与每条发言的生命周期分别建模。
│   ├── speech.rs  # 播报文本校验、任务快照、生成代次与执行状态语义
│   ├── training.rs  # 训练状态、受限素材、成对权重与音色绑定的领域不变量
│   └── voice.rs  # 配置音色标识校验
├── tests/  # 领域对象与输入不变量集成测试
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── event_validation.rs  # 直播事件输入边界测试
│   ├── resource_validation.rs  # 资源名称、语言、素材元数据与能力映射校验测试
│   └── speech_validation.rs  # 播报文本长度、空白、控制字符与音色标识校验测试
├── Cargo.toml  # 该 Rust 包的名称、workspace 配置与模块依赖声明
└── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
```

可继续查看各子目录的索引：

- [src/](src/DIRECTORY.md)：事件、会话、角色、音色和执行状态的领域定义
- [tests/](tests/DIRECTORY.md)：领域对象与输入不变量集成测试

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 78f8ffd5af31b6fe481d180c1ed87885e9bcecadae2ee402bb81d8e92ddf9e7b -->
