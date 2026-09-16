# apps 目录索引

可执行应用入口与依赖组装

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
apps/  # 可执行应用入口与依赖组装
├── desktop/  # React 控制面板及 Windows 桌面外壳
│   ├── src/  # 按应用组装、业务功能、外部服务和公共能力组织的前端源码
│   │   ├── app/  # React 根页面组装与全局样式
│   │   │   ├── resources/  # 角色与音色功能的页面组装和共享状态
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── ResourcesPanel.lifecycle.test.tsx  # 资源读取与操作的卸载取消测试
│   │   │   │   ├── ResourcesPanel.regressions.test.tsx  # 当前角色重新加载与安装成功后刷新失败回归
│   │   │   │   ├── ResourcesPanel.tsx  # 组装角色与音色面板并显示资源操作状态
│   │   │   │   ├── index.ts  # 资源管理页面公共入口
│   │   │   │   └── useResourcesController.ts  # 资源快照与桌面操作的共享控制器
│   │   │   ├── App.desktop.test.tsx  # 原生桌面地址初始化、导航功能请求地址与失败回归
│   │   │   ├── App.launcher.test.tsx  # 服务启停后的业务门控、环境页面常驻访问与模型检索草稿保留测试
│   │   │   ├── App.navigation.test.tsx  # 切页保留在途播报、历史深链接与未知页面回退的集成测试
│   │   │   ├── App.test.tsx  # 导航功能显隐、草稿保留与当前页标识的集成测试
│   │   │   ├── App.tsx  # 组装客户端与原生配置，选择受管或手动模式的导航控制台
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── ManagedWorkspace.tsx  # 共享启动管理状态、模型操作会话及业务就绪门控的工作区
│   │   │   ├── Workspace.tsx  # 固定侧栏、始终可访问的环境页面、快捷入口与稳定挂载的导航布局
│   │   │   ├── WorkspaceIcon.tsx  # 控制台导航、品牌猫形与快捷操作的代码内 SVG 图标
│   │   │   ├── navigation.ts  # 控制台功能导航、分组、页面说明与 URL fragment 映射
│   │   │   ├── styles.css  # 控制台公共样式、响应式布局与服务滑动开关样式
│   │   │   ├── useWorkspaceNavigation.ts  # 页面选择、访问记录与浏览器前进后退同步
│   │   │   └── workspace.css  # 粉色亚克力导航工作区、圆角下拉与文件选择控件、半透明卡片阴影和环境模型页面的响应式样式
│   │   ├── features/  # 面向用户的功能模块，各自封装组件与状态
│   │   │   ├── agent/  # Agent 人设、话题和互动策略设置
│   │   │   │   ├── AgentPanel.test.tsx  # Agent 状态控制、错误呈现、轮询竞态与取消清理测试
│   │   │   │   ├── AgentPanel.tsx  # Agent 运行条件、暂停恢复、设置、事件输入与历史的组合面板
│   │   │   │   ├── AgentSettingsForm.bounds.test.tsx  # 默认空话题及人设话题长度与后端一致性的回归测试
│   │   │   │   ├── AgentSettingsForm.test.tsx  # Agent 设置草稿、输入校验与毫秒请求映射测试
│   │   │   │   ├── AgentSettingsForm.tsx  # 保留草稿并以秒编辑冷却时间的 Agent 设置表单
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── EventHistory.test.tsx  # Agent 事件内容、状态、播报关联与错误展示测试
│   │   │   │   ├── EventHistory.tsx  # Agent 事件状态、关联播报及错误历史列表
│   │   │   │   ├── EventSimulator.test.tsx  # 模拟事件校验、成功清空、失败保留与重复反馈测试
│   │   │   │   ├── EventSimulator.tsx  # 聊天礼物模拟及 JSON 批量回放表单
│   │   │   │   ├── index.ts  # Agent 自动互动面板的功能出口
│   │   │   │   └── useAgentController.ts  # 修订号防回滚及卸载取消的串行 Agent 轮询与动作控制器
│   │   │   ├── characters/  # 角色模型选择、导入与动作映射界面
│   │   │   │   ├── CharacterPanel.test.tsx  # 角色导入、保存、加载与能力预览交互测试
│   │   │   │   ├── CharacterPanel.tsx  # 角色模型、音色、口型与热键映射管理界面
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── index.ts  # 角色管理：VTS 模型选择、表情动作映射及导入操作的界面。
│   │   │   │   └── types.ts  # 角色面板需要的状态与操作接口
│   │   │   ├── connections/  # 直播平台连接状态、事件计数和人工连接控制
│   │   │   │   ├── ConnectionPanel.test.tsx  # 直播连接面板状态展示、按钮规则与错误交互测试
│   │   │   │   ├── ConnectionPanel.tsx  # 直播平台连接状态、事件统计与手动连接控制面板
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── index.ts  # 直播平台连接功能公共出口
│   │   │   │   ├── polling.test.tsx  # 直播连接轮询串行、严格模式、迟到响应及卸载取消测试
│   │   │   │   └── useConnectionController.ts  # 直播连接轮询、操作互斥、取消及响应顺序控制器
│   │   │   ├── launcher/  # 控制面板服务开关、启动状态及新人引导
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── LauncherControls.tsx  # 主服务、TTS 和 Windows 执行端三开关、新人步骤与配置帮助
│   │   │   │   ├── LauncherPanel.test.tsx  # 滑动开关真实状态、启动取消、外部服务与断线交互测试
│   │   │   │   ├── LauncherPanel.tsx  # 服务开关展示与业务就绪门控的可复用组合入口
│   │   │   │   ├── index.ts  # 服务开关展示、状态控制器与组合面板公共导出
│   │   │   │   └── useLauncher.ts  # 启动管理状态轮询和串行启停请求，防止过期状态覆盖
│   │   │   ├── live/  # 直播工作台、弹幕观察与播报控制
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── SpeechPanel.scenarios.test.tsx  # 提交、停止、失败与重复操作场景测试
│   │   │   │   ├── SpeechPanel.test.tsx  # 输入校验、可用状态和历史展示组件测试
│   │   │   │   ├── SpeechPanel.tsx  # 中文人工语音播报控制面板
│   │   │   │   ├── index.ts  # 直播工作台：会话状态、弹幕观察、播放状态与人工控制。
│   │   │   │   ├── polling.test.tsx  # 状态刷新、取消清理与迟到响应场景测试
│   │   │   │   └── useSpeechController.ts  # 可取消的串行状态刷新与播报操作状态
│   │   │   ├── model-library/  # 环境检查、本地语音模型选择与官方模型下载管理界面
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── ModelLibraryPanel.test.tsx  # 环境门控、模型选择、分页检索、会话失效和下载取消交互测试
│   │   │   │   ├── ModelLibraryPanel.tsx  # 始终可用的环境与模型页面、分页模型库、下载进度及首次使用引导
│   │   │   │   └── useModelLibrary.ts  # 模型状态轮询与串行操作控制，防止过期响应覆盖和重复提交
│   │   │   ├── obs/  # OBS 场景与录制控制面板
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── ObsPanel.test.tsx  # OBS 显式控制、状态读回和失败后禁用操作的组件测试
│   │   │   │   ├── ObsPanel.tsx  # OBS 状态刷新、场景选择和人工录制控制界面
│   │   │   │   └── index.ts  # OBS 功能模块的公开组件出口
│   │   │   ├── training/  # 音色训练素材审核、任务版本和离线测量界面
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── TrainingPanel.saved.test.tsx  # 音色保存、重开复用、跨音色版本切换和失败重试交互回归测试
│   │   │   │   ├── TrainingPanel.test.tsx  # 训练提交审核、任务取消、试听确认和音频释放交互测试
│   │   │   │   ├── TrainingPanel.tsx  # 训练素材审核、任务取消、版本试听保存、音色选择切换及离线预设面板
│   │   │   │   ├── index.ts  # 训练面板公开组件导出
│   │   │   │   ├── useTraining.test.tsx  # 训练轮询独立更新、故障恢复与取消迟到结果测试
│   │   │   │   └── useTraining.ts  # 训练资源预设独立轮询、音色保存切换及取消生命周期
│   │   │   ├── voices/  # 参考素材、音色试听和训练任务界面
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── VoicePanel.test.tsx  # 音色上传、选择、试听与缺失资源交互测试
│   │   │   │   ├── VoicePanel.tsx  # 音色列表、参考音频上传校验、试听及已保存训练音色入口
│   │   │   │   ├── index.ts  # 音色管理：参考素材、试听与训练任务展示；不在浏览器执行模型推理。
│   │   │   │   ├── types.ts  # 音色面板需要的状态与操作接口
│   │   │   │   ├── wav.test.ts  # 参考音频格式、时长、容量与静音边界测试
│   │   │   │   └── wav.ts  # 浏览器端参考 PCM16 WAV 结构与有效性校验
│   │   │   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── services/  # 前端访问主服务、Linux 启动管理与 Windows 桌面能力的统一边界
│   │   │   ├── desktop/  # Tauri 命令客户端与桌面能力边界
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── index.test.ts  # 桌面 IPC 状态校验、来源约束和超时回归测试
│   │   │   │   └── index.ts  # 浏览器与 Tauri 环境识别及有界桌面状态读取
│   │   │   ├── launcher/  # 前端访问本机启动管理器的服务适配层
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── index.test.ts  # 启动客户端状态校验、操作拒绝、超时和不重试测试
│   │   │   │   └── index.ts  # 严格校验三服务状态并访问带令牌的本机启停接口
│   │   │   ├── model-library/  # 前端访问启动管理器语音模型能力的服务适配层
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── index.test.ts  # 模型服务契约、危险链接、下载拒绝与超时测试
│   │   │   │   └── index.ts  # 模型库契约校验、官方链接校验和带会话令牌的限时请求
│   │   │   ├── server/  # Rust 主服务 HTTP / WebSocket 客户端入口
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── agent.bounds.test.ts  # Agent 合法最大快照、冷却与礼物数量边界回归测试
│   │   │   │   ├── agent.failures.test.ts  # Agent 畸形响应、HTTP 错误、超时与取消测试
│   │   │   │   ├── agent.requests.test.ts  # Agent 查询、设置、暂停恢复与事件批量请求契约测试
│   │   │   │   ├── agent.ts  # 独立 Agent HTTP 客户端、超时取消与运行时响应校验
│   │   │   │   ├── failures.test.ts  # HTTP 错误、协议校验、取消和超时测试
│   │   │   │   ├── index.ts  # 带超时和取消的主服务 HTTP 客户端
│   │   │   │   ├── live.failures.test.ts  # 直播快照边界、HTTP 错误、网络失败、超时及取消测试
│   │   │   │   ├── live.requests.test.ts  # 直播连接查询、连接及断开请求契约测试
│   │   │   │   ├── live.ts  # 直播连接 HTTP 客户端、超时取消与运行时快照校验
│   │   │   │   ├── obs.test.ts  # OBS HTTP 响应校验、失败和超时且不重放控制请求的测试
│   │   │   │   ├── obs.ts  # OBS 主服务请求、超时处理和状态契约校验
│   │   │   │   ├── requests.test.ts  # HTTP 请求与成功响应测试
│   │   │   │   ├── resources.failures.test.ts  # 资源响应边界、请求失败、超时与取消测试
│   │   │   │   ├── resources.requests.test.ts  # 资源接口路径、JSON 与音频上传负载测试
│   │   │   │   ├── resources.ts  # 资源 HTTP 与桌面操作客户端及运行时响应校验
│   │   │   │   ├── responses.ts  # 生成契约的运行时响应校验与错误映射
│   │   │   │   ├── training.test.ts  # 训练契约、音色保存请求、测量证据、超时取消及试听响应测试
│   │   │   │   └── training.ts  # 训练音色保存与运行预设 HTTP 客户端、严格响应校验和取消超时
│   │   │   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── shared/  # 跨功能复用且不持有业务流程的前端能力
│   │   │   ├── lib/  # 与业务状态无关的公共纯函数
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   └── index.ts  # 跨功能复用的纯函数；与网络、Tauri、全局业务状态无关。
│   │   │   ├── ui/  # 纯展示公共组件
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   └── index.ts  # 跨功能复用的纯展示组件；不请求网络，也不导入 features。
│   │   │   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── test/  # 前端测试公共设施
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── agent-fixtures.ts  # Agent 面板测试的完整状态与事件夹具
│   │   │   ├── launcher-fixtures.ts  # 三服务启动管理状态的前端测试数据
│   │   │   ├── live-fixtures.ts  # 直播连接面板与服务客户端测试的完整快照夹具
│   │   │   ├── model-library-fixtures.ts  # 环境检测、已安装模型与可下载模型的前端测试数据
│   │   │   ├── resource-fixtures.ts  # 资源档案、模型、热键与音频测试样例
│   │   │   ├── server-fixtures.ts  # 主服务响应与异步请求测试夹具
│   │   │   └── setup.ts  # DOM 测试匹配器与清理
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   └── main.tsx  # 创建 React 根节点并挂载应用与全局样式
│   ├── src-tauri/  # Windows Tauri 薄外壳、窗口配置与执行库生命周期
│   │   ├── capabilities/  # 桌面窗口的 Tauri 能力声明
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── default.json  # 主窗口的最小权限范围，未开放系统插件能力
│   │   ├── icons/  # 沿用控制台绿色 M 标志的图标源及窗口构建资源
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── icon.ico  # Windows 窗口与应用资源使用的 ICO 图标
│   │   │   ├── icon.png  # Tauri 使用的 RGBA 窗口图标
│   │   │   └── icon.svg  # 绿色 M 应用图标的可编辑矢量源
│   │   ├── src/  # 桌面启动、依赖组装与命令转发源码
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── bootstrap.rs  # Windows Tauri 窗口组装、宿主启动及退出取消清理
│   │   │   ├── commands.rs  # 仅暴露配置地址与执行宿主状态的 Tauri IPC 命令
│   │   │   ├── lib.rs  # Windows 桌面外壳库入口，执行逻辑委托独立运行库
│   │   │   ├── main.rs  # 启动桌面外壳并报告启动失败的进程入口
│   │   │   └── startup.rs  # 桌面启动参数、首次配置创建及相对路径解析
│   │   ├── tests/  # 可跨平台验证的桌面配置与启动测试
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── configuration.rs  # 首次配置、保留用户设置、参数和相对路径解析测试
│   │   ├── Cargo.toml  # 该 Rust 包的名称、workspace 配置与模块依赖声明
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── README.md  # 桌面入口状态、Tauri 接入边界及 Windows 验证要求
│   │   ├── build.rs  # Windows Tauri 构建资源与权限生成入口
│   │   └── tauri.conf.json  # Tauri 窗口、构建地址、CSP 与后续安装包配置
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── index.html  # 控制面板 HTML 容器与前端脚本入口
│   ├── package.json  # React 控制面板的依赖、开发和构建命令
│   ├── tsconfig.json  # 前端 DOM、JSX 和源码范围的 TypeScript 配置
│   └── vite.config.ts  # Vite 开发服务及 Vitest DOM 测试环境
├── server/  # Linux / WSL 主服务入口、配置和传输边界
│   ├── src/  # 主服务启动、配置解析及 HTTP / WebSocket 适配源码
│   │   ├── agent/  # Agent 服务状态、跨端映射和异步模型调度
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── mapping.rs  # 统一事件及 Agent 业务状态到公开 HTTP DTO 的映射
│   │   │   ├── state.rs  # Agent 查询控制、原子事件接收及语音状态同步
│   │   │   └── worker.rs  # 锁外模型调用、有界重试、GPU 在途互斥及取消核对后的语音入队
│   │   ├── config/  # 按 Agent 与模型能力拆分的配置校验
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── agent.rs  # Agent 人设和有界调度参数的 TOML 配置
│   │   │   ├── live.rs  # 官方直播接入开关、应用编号、凭据变量名和重连参数校验
│   │   │   ├── llm.rs  # 模型地址、密钥环境变量名与调用资源上限校验
│   │   │   ├── resources.rs  # Linux 资源保存目录和引擎共享挂载配置
│   │   │   └── training.rs  # 训练路径、超时及受管推理默认权重配置校验
│   │   ├── live/  # 官方直播源组装及异步连接、接收、清理与重连驱动
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── bootstrap.rs  # 从主服务环境变量组装官方直播适配器
│   │   │   └── worker.rs  # 直播事件接收、去重入队、取消、会话清理和退避重连
│   │   ├── transport/  # 控制接口、跨端连接与协议到领域对象的转换
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── agent.rs  # Agent 状态控制与批量事件输入的 HTTP 边界
│   │   │   ├── bridge.rs  # 独立控制与音频发送循环、心跳及执行回执接收
│   │   │   ├── error.rs  # 稳定的结构化 HTTP 错误映射
│   │   │   ├── http.rs  # 组装播报、Agent、事件、直播连接及 WebSocket 路由和来源校验
│   │   │   ├── live.rs  # 直播连接查询、连接及断开的 HTTP 输入边界
│   │   │   ├── mapping.rs  # protocol DTO 与 domain 类型的显式转换，避免序列化字段影响领域规则。
│   │   │   ├── mod.rs  # HTTP / WebSocket 输入与输出适配；在协议 DTO 与领域对象之间进行映射。
│   │   │   ├── obs.rs  # 通过桌面资源通道执行 OBS 状态查询与受限控制
│   │   │   ├── origin.rs  # 浏览器请求来源校验及 HTTP 来源中间件
│   │   │   ├── resources.rs  # 音色上传、角色保存选择和逐项能力验证路由
│   │   │   ├── runtime.rs  # 本地 LLM 到 TTS 联合时延显存测量与验证状态接口
│   │   │   ├── training.rs  # 有界训练上传、后台任务、取消、成对权重试听保存启用接口
│   │   │   └── websocket.rs  # 唯一执行端连接准入及音频配对校验
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── agent.rs  # Agent 异步驱动与服务状态组装入口
│   │   ├── bootstrap.rs  # 读取私有配置，组装语音、LLM 和直播适配器并管理服务生命周期
│   │   ├── config.rs  # 主服务配置聚合、读取与启动前校验
│   │   ├── gpu.rs  # 训练试听测量独占租约、保留直播及 Agent 状态的音色切换准入与互斥测试
│   │   ├── lib.rs  # 可注入适配器的服务模块导出与集成测试入口
│   │   ├── live.rs  # 直播连接单会话所有权、公开快照和手动连接断开控制
│   │   ├── main.rs  # Linux / WSL 主服务入口。业务编排位于 meowlive-application。
│   │   ├── resources.rs  # 桌面资源请求关联、单操作准入与取消生命周期
│   │   ├── state.rs  # 语音、Agent、唯一执行桥接和直播连接的共享状态及取消生命周期
│   │   └── worker.rs  # 语音任务 I/O 驱动、PCM 分片传输与设备回执等待
│   ├── tests/  # 主服务配置、HTTP、桥接与完整播报集成测试
│   │   ├── agent_support/  # Agent 服务集成测试公共夹具
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── mod.rs  # 可注入模型与语音的服务和后台任务测试组装
│   │   ├── live_support/  # 可控直播源与事件的服务生命周期测试支持
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── mod.rs  # 可控直播连接、会话队列、事件与状态等待工具
│   │   ├── process_support/  # 真实主服务进程测试的隔离目录和音频构造辅助
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── mod.rs  # 主服务测试子进程启动清理和受控 WAV 构造
│   │   ├── support/  # 主服务集成测试公共夹具
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── mod.rs  # 受控合成器、HTTP 请求与双 WebSocket 连接夹具
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── agent_api.rs  # Agent 默认暂停、设置、事件校验、去重与停止接口测试
│   │   ├── agent_bootstrap.rs  # LLM 启动配置、缺失环境变量及无认证本地模型测试
│   │   ├── agent_cancellation.rs  # 暂停停止配置修改和断线取消在途 LLM 的集成测试
│   │   ├── agent_capacity.rs  # 事件批量请求大小和容量拒绝的原子性测试
│   │   ├── agent_configuration.rs  # Agent 和 LLM 的默认配置、字段边界及解析错误脱敏测试
│   │   ├── agent_process.rs  # 真实主服务进程、受控 LLM 与 TTS、静音客户端的播报闭环测试
│   │   ├── agent_receipt_retention.rs  # 语音历史裁剪时保留 Agent 已完成播放结果的回归测试
│   │   ├── agent_retries.rs  # 模型临时错误分类和有限重试次数的集成测试
│   │   ├── agent_runtime.rs  # 模拟事件到设备播放完成及断线未知状态的集成测试
│   │   ├── bridge_handshake.rs  # 协议版本、唯一执行端与双连接配对测试
│   │   ├── configuration.rs  # 示例配置兼容及无效参数拒绝测试
│   │   ├── http_api.rs  # 状态、播报输入与停止接口测试
│   │   ├── live_admission.rs  # 直播事件容量丢弃、无效事件及退出清理测试
│   │   ├── live_cleanup.rs  # 直播延迟清理、清理失败、房间状态与退避取消回归测试
│   │   ├── live_configuration.rs  # 直播凭据缺失、环境变量边界与配置验证测试
│   │   ├── live_http.rs  # 直播默认禁用状态、连接入口和配置范围测试
│   │   ├── live_lifecycle.rs  # 单直播连接、取消迟到结果、事件去重与重连终止测试
│   │   ├── live_process.rs  # 真实主服务进程的直播凭据组装、默认不连接与公开响应脱敏测试
│   │   ├── live_runtime.rs  # 受控平台礼物与重复帧经真实适配器、Agent、语音和静音设备完成回执的联调测试
│   │   ├── m5_config.rs  # 本地预设地址资源上限及训练配置约束测试
│   │   ├── obs_http.rs  # OBS HTTP 参数、桌面连接要求及模拟执行端贯通测试
│   │   ├── request_origin.rs  # HTTP 来源拒绝和无副作用保障测试
│   │   ├── resources_bridge.rs  # 桌面资源请求关联、停止、断连及响应边界测试
│   │   ├── resources_characters.rs  # 角色加载确认、预览验证及映射变更失效测试
│   │   ├── resources_http.rs  # 资源初始状态、当前音色别名与离线操作测试
│   │   ├── resources_process.rs  # 真实服务进程上传、恢复、试听、Agent 与缺失资源链路测试
│   │   ├── resources_runtime.rs  # 面板经真实桌面执行库到受控 VTS 的角色、热键与停止链路测试
│   │   ├── runtime_loop.rs  # 真实服务与独立桌面运行时的静音播放集成测试
│   │   ├── speech_delivery.rs  # PCM 下发、设备回执门控及独立停止通道测试
│   │   ├── synthesis_cancellation.rs  # 合成停止、断线未知与重连不重播集成测试
│   │   ├── training_http.rs  # 训练未配置、音色保存接口和离线未验证的公开 HTTP 行为测试
│   │   └── training_process.rs  # 真实主服务训练上传、音色保存重启、版本试听启用、取消及 SIGTERM 子树清理测试
│   ├── Cargo.toml  # 该 Rust 包的名称、workspace 配置与模块依赖声明
│   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
└── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
```

可继续查看各子目录的索引：

- [desktop/](desktop/DIRECTORY.md)：React 控制面板及 Windows 桌面外壳
- [server/](server/DIRECTORY.md)：Linux / WSL 主服务入口、配置和传输边界

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: c074d26a00cf875e2b679a84de1d2cf5af4ee378a538426405bb775a5650b386 -->
