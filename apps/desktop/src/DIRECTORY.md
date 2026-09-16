# src 目录索引

按应用组装、业务功能、外部服务和公共能力组织的前端源码

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
src/  # 按应用组装、业务功能、外部服务和公共能力组织的前端源码
├── app/  # React 根页面组装与全局样式
│   ├── resources/  # 角色与音色功能的页面组装和共享状态
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── ResourcesPanel.lifecycle.test.tsx  # 资源读取与操作的卸载取消测试
│   │   ├── ResourcesPanel.regressions.test.tsx  # 当前角色重新加载与安装成功后刷新失败回归
│   │   ├── ResourcesPanel.tsx  # 组装角色与音色面板并显示资源操作状态
│   │   ├── index.ts  # 资源管理页面公共入口
│   │   └── useResourcesController.ts  # 资源快照与桌面操作的共享控制器
│   ├── App.desktop.test.tsx  # 原生桌面地址初始化、导航功能请求地址与失败回归
│   ├── App.launcher.test.tsx  # 服务启停后的业务门控、环境页面常驻访问与模型检索草稿保留测试
│   ├── App.navigation.test.tsx  # 切页保留在途播报、历史深链接与未知页面回退的集成测试
│   ├── App.test.tsx  # 导航功能显隐、草稿保留与当前页标识的集成测试
│   ├── App.tsx  # 组装客户端与原生配置，选择受管或手动模式的导航控制台
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── ManagedWorkspace.tsx  # 共享启动管理状态、模型操作会话及业务就绪门控的工作区
│   ├── Workspace.tsx  # 固定侧栏、始终可访问的环境页面、快捷入口与稳定挂载的导航布局
│   ├── WorkspaceIcon.tsx  # 控制台导航、品牌猫形与快捷操作的代码内 SVG 图标
│   ├── navigation.ts  # 控制台功能导航、分组、页面说明与 URL fragment 映射
│   ├── styles.css  # 控制台公共样式、响应式布局与服务滑动开关样式
│   ├── useWorkspaceNavigation.ts  # 页面选择、访问记录与浏览器前进后退同步
│   └── workspace.css  # 粉色亚克力导航工作区、圆角下拉与文件选择控件、半透明卡片阴影和环境模型页面的响应式样式
├── features/  # 面向用户的功能模块，各自封装组件与状态
│   ├── agent/  # Agent 人设、话题和互动策略设置
│   │   ├── AgentPanel.test.tsx  # Agent 状态控制、错误呈现、轮询竞态与取消清理测试
│   │   ├── AgentPanel.tsx  # Agent 运行条件、暂停恢复、设置、事件输入与历史的组合面板
│   │   ├── AgentSettingsForm.bounds.test.tsx  # 默认空话题及人设话题长度与后端一致性的回归测试
│   │   ├── AgentSettingsForm.test.tsx  # Agent 设置草稿、输入校验与毫秒请求映射测试
│   │   ├── AgentSettingsForm.tsx  # 保留草稿并以秒编辑冷却时间的 Agent 设置表单
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── EventHistory.test.tsx  # Agent 事件内容、状态、播报关联与错误展示测试
│   │   ├── EventHistory.tsx  # Agent 事件状态、关联播报及错误历史列表
│   │   ├── EventSimulator.test.tsx  # 模拟事件校验、成功清空、失败保留与重复反馈测试
│   │   ├── EventSimulator.tsx  # 聊天礼物模拟及 JSON 批量回放表单
│   │   ├── index.ts  # Agent 自动互动面板的功能出口
│   │   └── useAgentController.ts  # 修订号防回滚及卸载取消的串行 Agent 轮询与动作控制器
│   ├── characters/  # 角色模型选择、导入与动作映射界面
│   │   ├── CharacterPanel.test.tsx  # 角色导入、保存、加载与能力预览交互测试
│   │   ├── CharacterPanel.tsx  # 角色模型、音色、口型与热键映射管理界面
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── index.ts  # 角色管理：VTS 模型选择、表情动作映射及导入操作的界面。
│   │   └── types.ts  # 角色面板需要的状态与操作接口
│   ├── connections/  # 直播平台连接状态、事件计数和人工连接控制
│   │   ├── ConnectionPanel.test.tsx  # 直播连接面板状态展示、按钮规则与错误交互测试
│   │   ├── ConnectionPanel.tsx  # 直播平台连接状态、事件统计与手动连接控制面板
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── index.ts  # 直播平台连接功能公共出口
│   │   ├── polling.test.tsx  # 直播连接轮询串行、严格模式、迟到响应及卸载取消测试
│   │   └── useConnectionController.ts  # 直播连接轮询、操作互斥、取消及响应顺序控制器
│   ├── launcher/  # 控制面板服务开关、启动状态及新人引导
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── LauncherControls.tsx  # 主服务、TTS 和 Windows 执行端三开关、新人步骤与配置帮助
│   │   ├── LauncherPanel.test.tsx  # 滑动开关真实状态、启动取消、外部服务与断线交互测试
│   │   ├── LauncherPanel.tsx  # 服务开关展示与业务就绪门控的可复用组合入口
│   │   ├── index.ts  # 服务开关展示、状态控制器与组合面板公共导出
│   │   └── useLauncher.ts  # 启动管理状态轮询和串行启停请求，防止过期状态覆盖
│   ├── live/  # 直播工作台、弹幕观察与播报控制
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── SpeechPanel.scenarios.test.tsx  # 提交、停止、失败与重复操作场景测试
│   │   ├── SpeechPanel.test.tsx  # 输入校验、可用状态和历史展示组件测试
│   │   ├── SpeechPanel.tsx  # 中文人工语音播报控制面板
│   │   ├── index.ts  # 直播工作台：会话状态、弹幕观察、播放状态与人工控制。
│   │   ├── polling.test.tsx  # 状态刷新、取消清理与迟到响应场景测试
│   │   └── useSpeechController.ts  # 可取消的串行状态刷新与播报操作状态
│   ├── model-library/  # 环境检查、本地语音模型选择与官方模型下载管理界面
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── ModelLibraryPanel.test.tsx  # 环境门控、模型选择、分页检索、会话失效和下载取消交互测试
│   │   ├── ModelLibraryPanel.tsx  # 始终可用的环境与模型页面、分页模型库、下载进度及首次使用引导
│   │   └── useModelLibrary.ts  # 模型状态轮询与串行操作控制，防止过期响应覆盖和重复提交
│   ├── obs/  # OBS 场景与录制控制面板
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── ObsPanel.test.tsx  # OBS 显式控制、状态读回和失败后禁用操作的组件测试
│   │   ├── ObsPanel.tsx  # OBS 状态刷新、场景选择和人工录制控制界面
│   │   └── index.ts  # OBS 功能模块的公开组件出口
│   ├── training/  # 音色训练素材审核、任务版本和离线测量界面
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── TrainingPanel.saved.test.tsx  # 音色保存、重开复用、跨音色版本切换和失败重试交互回归测试
│   │   ├── TrainingPanel.test.tsx  # 训练提交审核、任务取消、试听确认和音频释放交互测试
│   │   ├── TrainingPanel.tsx  # 训练素材审核、任务取消、版本试听保存、音色选择切换及离线预设面板
│   │   ├── index.ts  # 训练面板公开组件导出
│   │   ├── useTraining.test.tsx  # 训练轮询独立更新、故障恢复与取消迟到结果测试
│   │   └── useTraining.ts  # 训练资源预设独立轮询、音色保存切换及取消生命周期
│   ├── voices/  # 参考素材、音色试听和训练任务界面
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── VoicePanel.test.tsx  # 音色上传、选择、试听与缺失资源交互测试
│   │   ├── VoicePanel.tsx  # 音色列表、参考音频上传校验、试听及已保存训练音色入口
│   │   ├── index.ts  # 音色管理：参考素材、试听与训练任务展示；不在浏览器执行模型推理。
│   │   ├── types.ts  # 音色面板需要的状态与操作接口
│   │   ├── wav.test.ts  # 参考音频格式、时长、容量与静音边界测试
│   │   └── wav.ts  # 浏览器端参考 PCM16 WAV 结构与有效性校验
│   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── services/  # 前端访问主服务、Linux 启动管理与 Windows 桌面能力的统一边界
│   ├── desktop/  # Tauri 命令客户端与桌面能力边界
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── index.test.ts  # 桌面 IPC 状态校验、来源约束和超时回归测试
│   │   └── index.ts  # 浏览器与 Tauri 环境识别及有界桌面状态读取
│   ├── launcher/  # 前端访问本机启动管理器的服务适配层
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── index.test.ts  # 启动客户端状态校验、操作拒绝、超时和不重试测试
│   │   └── index.ts  # 严格校验三服务状态并访问带令牌的本机启停接口
│   ├── model-library/  # 前端访问启动管理器语音模型能力的服务适配层
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── index.test.ts  # 模型服务契约、危险链接、下载拒绝与超时测试
│   │   └── index.ts  # 模型库契约校验、官方链接校验和带会话令牌的限时请求
│   ├── server/  # Rust 主服务 HTTP / WebSocket 客户端入口
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── agent.bounds.test.ts  # Agent 合法最大快照、冷却与礼物数量边界回归测试
│   │   ├── agent.failures.test.ts  # Agent 畸形响应、HTTP 错误、超时与取消测试
│   │   ├── agent.requests.test.ts  # Agent 查询、设置、暂停恢复与事件批量请求契约测试
│   │   ├── agent.ts  # 独立 Agent HTTP 客户端、超时取消与运行时响应校验
│   │   ├── failures.test.ts  # HTTP 错误、协议校验、取消和超时测试
│   │   ├── index.ts  # 带超时和取消的主服务 HTTP 客户端
│   │   ├── live.failures.test.ts  # 直播快照边界、HTTP 错误、网络失败、超时及取消测试
│   │   ├── live.requests.test.ts  # 直播连接查询、连接及断开请求契约测试
│   │   ├── live.ts  # 直播连接 HTTP 客户端、超时取消与运行时快照校验
│   │   ├── obs.test.ts  # OBS HTTP 响应校验、失败和超时且不重放控制请求的测试
│   │   ├── obs.ts  # OBS 主服务请求、超时处理和状态契约校验
│   │   ├── requests.test.ts  # HTTP 请求与成功响应测试
│   │   ├── resources.failures.test.ts  # 资源响应边界、请求失败、超时与取消测试
│   │   ├── resources.requests.test.ts  # 资源接口路径、JSON 与音频上传负载测试
│   │   ├── resources.ts  # 资源 HTTP 与桌面操作客户端及运行时响应校验
│   │   ├── responses.ts  # 生成契约的运行时响应校验与错误映射
│   │   ├── training.test.ts  # 训练契约、音色保存请求、测量证据、超时取消及试听响应测试
│   │   └── training.ts  # 训练音色保存与运行预设 HTTP 客户端、严格响应校验和取消超时
│   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── shared/  # 跨功能复用且不持有业务流程的前端能力
│   ├── lib/  # 与业务状态无关的公共纯函数
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   └── index.ts  # 跨功能复用的纯函数；与网络、Tauri、全局业务状态无关。
│   ├── ui/  # 纯展示公共组件
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   └── index.ts  # 跨功能复用的纯展示组件；不请求网络，也不导入 features。
│   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── test/  # 前端测试公共设施
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── agent-fixtures.ts  # Agent 面板测试的完整状态与事件夹具
│   ├── launcher-fixtures.ts  # 三服务启动管理状态的前端测试数据
│   ├── live-fixtures.ts  # 直播连接面板与服务客户端测试的完整快照夹具
│   ├── model-library-fixtures.ts  # 环境检测、已安装模型与可下载模型的前端测试数据
│   ├── resource-fixtures.ts  # 资源档案、模型、热键与音频测试样例
│   ├── server-fixtures.ts  # 主服务响应与异步请求测试夹具
│   └── setup.ts  # DOM 测试匹配器与清理
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
└── main.tsx  # 创建 React 根节点并挂载应用与全局样式
```

可继续查看各子目录的索引：

- [app/](app/DIRECTORY.md)：React 根页面组装与全局样式
- [features/](features/DIRECTORY.md)：面向用户的功能模块，各自封装组件与状态
- [services/](services/DIRECTORY.md)：前端访问主服务、Linux 启动管理与 Windows 桌面能力的统一边界
- [shared/](shared/DIRECTORY.md)：跨功能复用且不持有业务流程的前端能力
- [test/](test/DIRECTORY.md)：前端测试公共设施

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: c4fc5bf0cf9476c5336b482e880d130e7948ebd55d55f60a0a83bb48c49e250f -->
