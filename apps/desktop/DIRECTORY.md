# desktop 目录索引

React 控制面板及 Windows 桌面外壳

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
desktop/  # React 控制面板及 Windows 桌面外壳
├── src/  # 按应用组装、业务功能、外部服务和公共能力组织的前端源码
│   ├── app/  # React 根页面组装与全局样式
│   │   ├── feedback/  # 全局操作结果弹窗、去重与面板反馈回归测试
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── OperationFeedback.test.tsx  # 结果队列、焦点恢复、后台故障去重及启动状态回归测试
│   │   │   ├── OperationFeedback.tsx  # 全局操作结果队列、错误去重、无障碍弹窗与焦点恢复
│   │   │   ├── PanelFeedback.test.tsx  # 各控制面板操作结果、业务失败、异步状态与输入校验弹窗测试
│   │   │   └── feedback.css  # 全局结果弹窗的醒目配色、遮罩与响应式样式
│   │   ├── resources/  # 角色与音色功能的页面组装和共享状态
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── ResourcesPanel.lifecycle.test.tsx  # 资源读取与操作的卸载取消测试
│   │   │   ├── ResourcesPanel.preview.test.tsx  # 音色试听状态跟踪、执行端断线、失败与重试的前端回归测试
│   │   │   ├── ResourcesPanel.regressions.test.tsx  # 当前角色重新加载与安装成功后刷新失败回归
│   │   │   ├── ResourcesPanel.tsx  # 组装角色与音色面板并显示资源操作状态
│   │   │   ├── index.ts  # 资源管理页面公共入口
│   │   │   └── useResourcesController.ts  # 角色音色快照、资源删除及桌面模型管理共享控制器
│   │   ├── App.desktop.test.tsx  # 原生桌面地址初始化、导航功能请求地址与失败回归
│   │   ├── App.launcher.test.tsx  # 服务启停后的业务门控、环境页面常驻访问与模型检索草稿保留测试
│   │   ├── App.navigation.test.tsx  # 切页保留在途播报、历史深链接与未知页面回退的集成测试
│   │   ├── App.test.tsx  # 导航功能显隐、草稿保留与当前页标识的集成测试
│   │   ├── App.tsx  # 组装客户端与原生配置，选择受管或手动模式的导航控制台
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── ManagedWorkspace.tsx  # 受管服务状态、模型启用提示与功能工作区集成
│   │   ├── Workspace.tsx  # 固定侧栏、始终可访问的环境页面、快捷入口与稳定挂载的导航布局
│   │   ├── WorkspaceIcon.tsx  # 控制台导航、品牌猫形与快捷操作的代码内 SVG 图标
│   │   ├── navigation.ts  # 控制台功能导航、分组、页面说明与 URL fragment 映射
│   │   ├── styles.css  # 控制台公共样式、响应式布局与服务滑动开关样式
│   │   ├── useWorkspaceNavigation.ts  # 页面选择、访问记录与浏览器前进后退同步
│   │   └── workspace.css  # 工作区布局、模型库与训练页签分页及结果弹窗样式
│   ├── features/  # 面向用户的功能模块，各自封装组件与状态
│   │   ├── agent/  # Agent 人设、话题和互动策略设置
│   │   │   ├── AgentPanel.test.tsx  # Agent 状态控制、错误呈现、轮询竞态与取消清理测试
│   │   │   ├── AgentPanel.tsx  # Agent 运行条件、暂停恢复、设置、事件输入与历史的组合面板
│   │   │   ├── AgentSettingsForm.bounds.test.tsx  # 默认空话题及人设话题长度与后端一致性的回归测试
│   │   │   ├── AgentSettingsForm.test.tsx  # Agent 设置草稿、输入校验与毫秒请求映射测试
│   │   │   ├── AgentSettingsForm.tsx  # 保留草稿并以秒编辑冷却时间的 Agent 设置表单
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── EventHistory.test.tsx  # Agent 事件内容、状态、播报关联与错误展示测试
│   │   │   ├── EventHistory.tsx  # Agent 事件状态、关联播报及错误历史列表
│   │   │   ├── EventSimulator.test.tsx  # 模拟事件校验、成功清空、失败保留与重复反馈测试
│   │   │   ├── EventSimulator.tsx  # 聊天礼物模拟及 JSON 批量回放表单
│   │   │   ├── index.ts  # Agent 自动互动面板的功能出口
│   │   │   └── useAgentController.ts  # 修订号防回滚及卸载取消的串行 Agent 轮询与动作控制器
│   │   ├── characters/  # 角色模型选择、导入与动作映射界面
│   │   │   ├── CharacterPanel.test.tsx  # 角色与安装模型删除确认、导入保存加载及能力预览交互测试
│   │   │   ├── CharacterPanel.tsx  # 角色配置及安装模型增删、音色绑定、口型和热键管理界面
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── index.ts  # 角色管理：VTS 模型选择、表情动作映射及导入操作的界面。
│   │   │   └── types.ts  # 角色档案与本机模型增删管理界面的状态和能力契约
│   │   ├── connections/  # 直播平台连接状态、事件计数和人工连接控制
│   │   │   ├── ConnectionPanel.test.tsx  # 直播连接面板状态展示、按钮规则与错误交互测试
│   │   │   ├── ConnectionPanel.tsx  # 直播平台连接状态、事件统计与手动连接控制面板
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── index.ts  # 直播平台连接功能公共出口
│   │   │   ├── polling.test.tsx  # 直播连接轮询串行、严格模式、迟到响应及卸载取消测试
│   │   │   └── useConnectionController.ts  # 直播连接轮询、操作互斥、取消及响应顺序控制器
│   │   ├── launcher/  # 控制面板服务开关、启动状态及新人引导
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── LauncherControls.tsx  # 主服务、TTS 和 Windows 执行端三开关、新人步骤与配置帮助
│   │   │   ├── LauncherPanel.test.tsx  # 滑动开关真实状态、启动取消、外部服务与断线交互测试
│   │   │   ├── LauncherPanel.tsx  # 服务开关展示与业务就绪门控的可复用组合入口
│   │   │   ├── index.ts  # 服务开关展示、状态控制器与组合面板公共导出
│   │   │   └── useLauncher.ts  # 启动管理状态轮询和串行启停请求，防止过期状态覆盖
│   │   ├── live/  # 直播工作台、弹幕观察与播报控制
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── SpeechPanel.scenarios.test.tsx  # 提交、停止、失败与重复操作场景测试
│   │   │   ├── SpeechPanel.test.tsx  # 输入校验、可用状态和历史展示组件测试
│   │   │   ├── SpeechPanel.tsx  # 中文人工语音播报控制面板
│   │   │   ├── index.ts  # 直播工作台：会话状态、弹幕观察、播放状态与人工控制。
│   │   │   ├── polling.test.tsx  # 状态刷新、取消清理与迟到响应场景测试
│   │   │   └── useSpeechController.ts  # 可取消的串行状态刷新与播报操作状态
│   │   ├── llm/  # LLM 接入配置功能
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── LlmPanel.test.tsx  # LLM 面板草稿、密钥与生命周期测试
│   │   │   ├── LlmPanel.tsx  # LLM 服务商、协议、模型、密钥与连接测试面板
│   │   │   └── index.ts  # LLM 功能公共入口
│   │   ├── model-library/  # 环境检查、本地语音模型选择与官方模型下载管理界面
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── ModelLibraryPanel.test.tsx  # 环境门控、模型选择、分页检索、会话失效和下载取消交互测试
│   │   │   ├── ModelLibraryPanel.tsx  # 始终可用的环境与模型页面、分页模型库、下载进度及首次使用引导
│   │   │   └── useModelLibrary.ts  # 模型状态轮询与串行操作控制，防止过期响应覆盖和重复提交
│   │   ├── obs/  # OBS 场景与录制控制面板
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── ObsPanel.test.tsx  # OBS 显式控制、状态读回和失败后禁用操作的组件测试
│   │   │   ├── ObsPanel.tsx  # OBS 状态刷新、场景选择和人工录制控制界面
│   │   │   └── index.ts  # OBS 功能模块的公开组件出口
│   │   ├── training/  # 仅音频与可选文本训练、转写校对、任务版本和离线测量界面
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── TrainingPanel.audio.test.tsx  # 训练片段多格式转换上传、异步重选、容量与导入失败回归测试
│   │   │   ├── TrainingPanel.feedback.test.tsx  # 验证 GPU 失败提示、训练与测量结果弹窗、终态竞态去重及过期请求隔离
│   │   │   ├── TrainingPanel.saved.test.tsx  # 音色保存重开切换、删除确认及清理失败重试交互测试
│   │   │   ├── TrainingPanel.test.tsx  # 训练配置独立选择、服务忙碌时编辑、提交审核与试听取消交互测试
│   │   │   ├── TrainingPanel.transcription.test.tsx  # 训练声音与文本模式、空白转写汇总弹窗、审核及过期请求回归测试
│   │   │   ├── TrainingPanel.tsx  # 训练性能设置、记录分页、音色版本、模型开关及结果弹窗工作区
│   │   │   ├── TrainingPanel.workspace.test.tsx  # 训练页签与素材分页、音色归组、性能参数及独立模型启停测试
│   │   │   ├── TrainingResultDialog.tsx  # 训练成功失败结果弹窗、建议提示及键盘焦点恢复
│   │   │   ├── TrainingVoiceLibrary.tsx  # 按参考音色归组的训练音色库、版本选择与单版本操作
│   │   │   ├── index.ts  # 训练面板公开组件导出
│   │   │   ├── trainingFeedback.ts  # 训练操作结果、任务终态通知及失败原因对应的改正建议
│   │   │   ├── useTraining.test.tsx  # 训练轮询独立更新、故障恢复与取消迟到结果测试
│   │   │   └── useTraining.ts  # 训练资源与模型状态轮询、操作反馈、终态通知及异步取消保护
│   │   ├── voices/  # 参考素材、音色试听和训练任务界面
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── VoicePanel.test.tsx  # 多格式音色上传选择试听、删除确认失败及空状态交互测试
│   │   │   ├── VoicePanel.tsx  # 音色导入校验、选择试听和删除管理及训练音色入口
│   │   │   ├── index.ts  # 音色管理：参考素材、试听与训练任务展示；不在浏览器执行模型推理。
│   │   │   └── types.ts  # 音色列表选择试听、上传删除及文件清理重试能力契约
│   │   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── services/  # 前端访问主服务、Linux 启动管理与 Windows 桌面能力的统一边界
│   │   ├── audio/  # 参考音频与训练片段的浏览器解码、格式转换和音频校验
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── index.test.ts  # 多格式音频转换、PCM 编码、文件大小、时长与解码失败测试
│   │   │   ├── index.ts  # MP3 等多格式音频导入、离线解码与兼容 PCM16 WAV 转换
│   │   │   ├── wav.test.ts  # 参考与训练音频的 WAV 格式、时长、容量与静音边界测试
│   │   │   └── wav.ts  # 参考音频与训练片段的 PCM16 WAV 结构及有效性校验
│   │   ├── desktop/  # Tauri 命令客户端与桌面能力边界
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── index.test.ts  # 桌面 IPC 状态校验、来源约束和超时回归测试
│   │   │   └── index.ts  # 浏览器与 Tauri 环境识别及有界桌面状态读取
│   │   ├── launcher/  # 前端访问本机启动管理器的服务适配层
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── index.test.ts  # 启动客户端状态校验、操作拒绝、超时和不重试测试
│   │   │   └── index.ts  # 严格校验三服务状态并访问带令牌的本机启停接口
│   │   ├── model-library/  # 前端访问启动管理器语音模型能力的服务适配层
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── index.test.ts  # 模型服务契约、危险链接、下载拒绝与超时测试
│   │   │   └── index.ts  # 模型库契约校验、官方链接校验和带会话令牌的限时请求
│   │   ├── server/  # Rust 主服务 HTTP / WebSocket 客户端入口
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── agent.bounds.test.ts  # Agent 合法最大快照、冷却与礼物数量边界回归测试
│   │   │   ├── agent.failures.test.ts  # Agent 畸形响应、HTTP 错误、超时与取消测试
│   │   │   ├── agent.requests.test.ts  # Agent 查询、设置、暂停恢复与事件批量请求契约测试
│   │   │   ├── agent.ts  # 独立 Agent HTTP 客户端、超时取消与运行时响应校验
│   │   │   ├── failures.test.ts  # HTTP 错误、协议校验、取消和超时测试
│   │   │   ├── index.ts  # 带超时和取消的主服务 HTTP 客户端
│   │   │   ├── live.failures.test.ts  # 直播快照边界、HTTP 错误、网络失败、超时及取消测试
│   │   │   ├── live.requests.test.ts  # 直播连接查询、连接及断开请求契约测试
│   │   │   ├── live.ts  # 直播连接 HTTP 客户端、超时取消与运行时快照校验
│   │   │   ├── llm.test.ts  # LLM 客户端路由、校验、错误与超时测试
│   │   │   ├── llm.ts  # LLM 设置与连接测试 HTTP 客户端及响应校验
│   │   │   ├── obs.test.ts  # OBS HTTP 响应校验、失败和超时且不重放控制请求的测试
│   │   │   ├── obs.ts  # OBS 主服务请求、超时处理和状态契约校验
│   │   │   ├── requests.test.ts  # HTTP 请求与成功响应测试
│   │   │   ├── resources.failures.test.ts  # 资源响应边界、请求失败、超时与取消测试
│   │   │   ├── resources.requests.test.ts  # 资源增删接口负载、空绑定及安装模型响应关联测试
│   │   │   ├── resources.ts  # 资源 HTTP 与桌面操作客户端及运行时响应校验
│   │   │   ├── responses.ts  # 生成契约的运行时响应校验与错误映射
│   │   │   ├── training.test.ts  # 训练与转写契约、请求校验、保存删除及取消超时测试
│   │   │   └── training.ts  # 训练性能、任务版本、模型启停与离线测量客户端及响应校验
│   │   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── shared/  # 跨功能复用且不持有业务流程的前端能力
│   │   ├── lib/  # 与业务状态无关的公共纯函数
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── index.ts  # 跨功能复用的纯函数；与网络、Tauri、全局业务状态无关。
│   │   ├── ui/  # 纯展示公共组件
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── index.ts  # 跨功能复用的纯展示组件；不请求网络，也不导入 features。
│   │   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── test/  # 前端测试公共设施
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── agent-fixtures.ts  # Agent 面板测试的完整状态与事件夹具
│   │   ├── launcher-fixtures.ts  # 三服务启动管理状态的前端测试数据
│   │   ├── live-fixtures.ts  # 直播连接面板与服务客户端测试的完整快照夹具
│   │   ├── model-library-fixtures.ts  # 环境检测、已安装模型与可下载模型的前端测试数据
│   │   ├── resource-fixtures.ts  # 资源档案、模型、热键与音频测试样例
│   │   ├── server-fixtures.ts  # 主服务响应与异步请求测试夹具
│   │   └── setup.ts  # DOM 测试匹配器与清理
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── main.tsx  # 创建 React 根节点并挂载应用与全局样式
├── src-tauri/  # Windows Tauri 薄外壳、窗口配置与执行库生命周期
│   ├── capabilities/  # 桌面窗口的 Tauri 能力声明
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   └── default.json  # 主窗口的最小权限范围，未开放系统插件能力
│   ├── icons/  # 沿用控制台绿色 M 标志的图标源及窗口构建资源
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── icon.ico  # Windows 窗口与应用资源使用的 ICO 图标
│   │   ├── icon.png  # Tauri 使用的 RGBA 窗口图标
│   │   └── icon.svg  # 绿色 M 应用图标的可编辑矢量源
│   ├── src/  # 桌面启动、依赖组装与命令转发源码
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── bootstrap.rs  # Windows Tauri 窗口组装、宿主启动及退出取消清理
│   │   ├── commands.rs  # 仅暴露配置地址与执行宿主状态的 Tauri IPC 命令
│   │   ├── lib.rs  # Windows 桌面外壳库入口，执行逻辑委托独立运行库
│   │   ├── main.rs  # 启动桌面外壳并报告启动失败的进程入口
│   │   └── startup.rs  # 桌面启动参数、首次配置创建及相对路径解析
│   ├── tests/  # 可跨平台验证的桌面配置与启动测试
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   └── configuration.rs  # 首次配置、保留用户设置、参数和相对路径解析测试
│   ├── Cargo.toml  # 该 Rust 包的名称、workspace 配置与模块依赖声明
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── README.md  # 桌面入口状态、Tauri 接入边界及 Windows 验证要求
│   ├── build.rs  # Windows Tauri 构建资源与权限生成入口
│   └── tauri.conf.json  # Tauri 窗口、构建地址、CSP 与后续安装包配置
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── index.html  # 控制面板 HTML 容器与前端脚本入口
├── package.json  # React 控制面板的依赖、开发和构建命令
├── tsconfig.json  # 前端 DOM、JSX 和源码范围的 TypeScript 配置
└── vite.config.ts  # Vite 开发服务及 Vitest DOM 测试环境
```

可继续查看各子目录的索引：

- [src/](src/DIRECTORY.md)：按应用组装、业务功能、外部服务和公共能力组织的前端源码
- [src-tauri/](src-tauri/DIRECTORY.md)：Windows Tauri 薄外壳、窗口配置与执行库生命周期

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: ef961d1c95f1341615be28c262535f08088b4bce10601530cb4f215ff409a5bf -->
