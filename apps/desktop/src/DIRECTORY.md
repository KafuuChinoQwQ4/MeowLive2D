# src 目录索引

按应用组装、业务功能、外部服务和公共能力组织的前端源码

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
src/  # 按应用组装、业务功能、外部服务和公共能力组织的前端源码
├── app/  # React 根页面组装与全局样式
│   ├── feedback/  # 全局操作结果弹窗、去重与面板反馈回归测试
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── OperationFeedback.test.tsx  # 行内结果隔离、错误队列焦点恢复、后台故障去重及启动状态回归测试
│   │   ├── OperationFeedback.tsx  # 分页面行内反馈、错误去重队列、无障碍错误弹窗与焦点恢复
│   │   ├── PanelFeedback.test.tsx  # 各控制面板成功行内反馈、业务失败、异步状态与输入校验弹窗测试
│   │   └── feedback.css  # 页面行内结果文本及错误弹窗的配色、遮罩与响应式样式
│   ├── guide/  # 控制面板集中使用指南与各功能操作步骤
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── GuidePanel.tsx  # 独立新手指南、可展开的功能用法与工作区跳转入口
│   │   └── content.ts  # 启动、语音、角色、互动、Agent 观察、观众、直播和训练的中文使用步骤
│   ├── resources/  # 角色人物卡与音色页面组装、共享资源控制器
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── ResourcesPanel.lifecycle.test.tsx  # 资源读取与操作取消及训练选用后的音色刷新测试
│   │   ├── ResourcesPanel.preview.test.tsx  # 音色试听状态跟踪、执行端断线、失败与重试的前端回归测试
│   │   ├── ResourcesPanel.regressions.test.tsx  # 当前角色重新加载与安装成功后刷新失败回归
│   │   ├── ResourcesPanel.tsx  # 按角色或音色模式组装人物卡、资源面板及切页和训练操作后的刷新
│   │   ├── index.ts  # 资源管理页面公共入口
│   │   └── useResourcesController.ts  # 角色音色快照、切页刷新竞态、资源删除及桌面模型管理控制器
│   ├── AdminGate.test.tsx  # 认证启停、登录退出、错误重试与过期响应竞态回归测试
│   ├── AdminGate.tsx  # 默认直接访问控制面板及显式认证部署的会话检查、登录退出与访问门禁
│   ├── App.desktop.test.tsx  # 原生桌面地址初始化、导航功能请求地址与失败回归
│   ├── App.launcher.test.tsx  # 服务启停后的业务门控、环境页面常驻访问与模型检索草稿保留测试
│   ├── App.navigation.test.tsx  # 切页保留在途播报、历史深链接、观察页导航与未知页面回退的集成测试
│   ├── App.organization.test.tsx  # 导航分组、同页编辑保存、切页音色刷新及迟到响应集成测试
│   ├── App.test.tsx  # 导航功能显隐、草稿保留与当前页标识的集成测试
│   ├── App.tsx  # 组装业务与观察客户端、角色音色训练同步及受管或手动导航控制台
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── ManagedWorkspace.tsx  # 受管服务状态、模型启用提示与功能工作区集成
│   ├── Workspace.guide.test.tsx  # 服务未就绪时访问指南、指南跳转与草稿保留的回归测试
│   ├── Workspace.tsx  # 分组侧栏、快捷入口、稳定挂载及当前页面上下文的导航布局
│   ├── WorkspaceIcon.tsx  # 控制台导航、品牌猫形与快捷操作的代码内 SVG 图标
│   ├── navigation.ts  # 控制台功能导航、分组、页面说明与 URL fragment 映射
│   ├── styles.css  # 控制台公共样式、响应式布局与服务滑动开关样式
│   ├── useWorkspaceNavigation.ts  # 页面选择、访问记录与浏览器前进后退同步
│   └── workspace.css  # 柔和工作区主题、统一下拉控件、人物卡、响应式布局与训练、使用指南和连接配置样式
├── features/  # 面向用户的功能模块，各自封装组件与状态
│   ├── agent/  # Agent 人设、话题和互动策略设置
│   │   ├── AgentPanel.test.tsx  # Agent 状态控制、最新人物卡合并、轮询竞态与取消清理测试
│   │   ├── AgentPanel.tsx  # Agent 运行条件、暂停恢复、互动设置、事件输入与历史的组合面板
│   │   ├── AgentSettingsForm.interaction.test.tsx  # 互动策略表单持久化、草稿保留与阈值边界测试
│   │   ├── AgentSettingsForm.test.tsx  # 互动设置保存、草稿保留、输入校验及读取失败反馈测试
│   │   ├── AgentSettingsForm.tsx  # 编辑话题、冷却及弹幕欢迎策略并保留人物卡的 Agent 互动表单
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── EventHistory.test.tsx  # Agent 事件内容、状态、播报关联与错误展示测试
│   │   ├── EventHistory.tsx  # Agent 事件状态、关联播报及错误历史列表
│   │   ├── EventSimulator.test.tsx  # 模拟事件校验、成功清空、失败保留与重复反馈测试
│   │   ├── EventSimulator.tsx  # 聊天、礼物、SC 与进房模拟及 JSON 批量回放表单
│   │   ├── PersonaCardPanel.bounds.test.tsx  # 人物卡必填身份、完整提示词 Unicode 长度及旧人设兼容测试
│   │   ├── PersonaCardPanel.test.tsx  # 人物卡结构化提示词、字段还原、最新互动设置合并及保存确认测试
│   │   ├── PersonaCardPanel.tsx  # 角色页人物卡编辑、Unicode 校验与合并最新互动设置后保存
│   │   ├── index.ts  # Agent 自动互动面板的功能出口
│   │   ├── personaCard.test.ts  # 人物卡标签转义、粘贴内容无损还原及非标准旧人设兼容测试
│   │   ├── personaCard.ts  # 人物卡字段模型及兼容旧纯文本人设的提示词序列化与还原
│   │   └── useAgentController.ts  # 修订号防回滚及卸载取消的串行 Agent 轮询与动作控制器
│   ├── agent-observability/  # Agent 调度原因、Trace 历史、模型 Turn 和执行时间线观察页面
│   │   ├── AgentObservabilityPanel.test.tsx  # 调度时间线、历史选择、隐私边界、可见性轮询与请求取消测试
│   │   ├── AgentObservabilityPanel.tsx  # Agent 调度快照、Trace 列表、模型 Turn 与执行步骤观察面板
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── agent-observability.css  # Agent 观察页的双栏历史时间线、状态徽章与窄屏布局样式
│   │   └── index.ts  # Agent 观察功能组件导出入口
│   ├── characters/  # 角色模型选择、导入与动作映射界面
│   │   ├── CharacterPanel.test.tsx  # 角色与安装模型删除确认、导入保存加载及能力预览交互测试
│   │   ├── CharacterPanel.tsx  # 角色配置及安装模型增删、音色绑定、口型和热键管理界面
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── index.ts  # 角色管理：VTS 模型选择、表情动作映射及导入操作的界面。
│   │   └── types.ts  # 角色档案与本机模型增删管理界面的状态和能力契约
│   ├── connections/  # 直播平台凭据配置、连接控制与运行状态展示
│   │   ├── ConnectionPanel.test.tsx  # 直播连接面板状态展示、按钮规则与错误交互测试
│   │   ├── ConnectionPanel.tsx  # 直播凭据配置入口、连接控制、运行计数与错误反馈面板
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── LiveSettingsForm.test.tsx  # 直播配置读写、凭据保留清除、校验及连接联动测试
│   │   ├── LiveSettingsForm.tsx  # 哔哩哔哩直播凭据配置表单、已保存提示和保存反馈
│   │   ├── index.ts  # 直播平台连接功能公共出口
│   │   ├── polling.test.tsx  # 直播连接轮询串行、严格模式、迟到响应及卸载取消测试
│   │   └── useConnectionController.ts  # 直播连接轮询、操作互斥、取消及响应顺序控制器
│   ├── launcher/  # 控制面板服务开关、启动状态及新人引导
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── LauncherControls.tsx  # 主服务、TTS 和 Windows 执行端三开关、状态及本机配置位置
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
│   ├── llm/  # LLM 接入配置功能
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── LlmPanel.models.test.tsx  # LLM 模型发现、自动识别、选择与连接切换交互测试
│   │   ├── LlmPanel.reasoning.test.tsx  # 推理档位选择、映射展示、预算阻断与请求竞态回归测试
│   │   ├── LlmPanel.test.tsx  # LLM 面板草稿、密钥与生命周期测试
│   │   ├── LlmPanel.tsx  # LLM 连接配置、模型获取与选择、推理档位预览和测试保存面板
│   │   ├── index.ts  # LLM 功能公共入口
│   │   └── useReasoningPreview.ts  # 推理能力预览的防抖、取消、重试与迟到响应隔离
│   ├── llm-runtime/  # 模型运行能力、用量费用与 Agent 实时活动界面
│   │   ├── AgentActivityPanel.test.tsx  # 活动更新和隐藏卸载取消轮询测试
│   │   ├── AgentActivityPanel.tsx  # Agent 阶段、工具进度与网页来源展示
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── LlmRuntimePanel.test.tsx  # 运行设置、密钥绑定、费用和取消交互测试
│   │   ├── LlmRuntimePanel.tsx  # 运行配置与用量折叠区入口
│   │   ├── RuntimeIntegration.test.tsx  # LLM 与 Agent 原页面运行能力接入测试
│   │   ├── RuntimeSettingsPanel.tsx  # 运行开关、搜索密钥及模型单价编辑与保存
│   │   ├── UsagePanel.tsx  # 按日期和模型查询已报告用量与估算费用
│   │   ├── llm-runtime.css  # 运行能力与用量界面的粉色响应式样式
│   │   ├── runtime-fixtures.ts  # 运行配置、用量及活动测试样例
│   │   └── useRuntimeVisibility.ts  # 按工作区和文档可见性暂停运行请求
│   ├── model-library/  # 环境检查、本地语音模型选择与官方模型下载管理界面
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── InstalledModels.tsx  # 同页分组展示声音生成与语音识别模型、独立选择及下载引导
│   │   ├── ModelLibraryPanel.test.tsx  # 环境门控、模型选择、分页检索、会话失效和下载取消交互测试
│   │   ├── ModelLibraryPanel.tsx  # 环境检测、音色训练与转文本分类模型库、用途与接入状态筛选及独立分页下载
│   │   └── useModelLibrary.ts  # 模型状态轮询与串行操作控制，防止过期响应覆盖和重复提交
│   ├── obs/  # OBS 场景与录制控制面板
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── ObsPanel.test.tsx  # OBS 设置读写、密码清除、旧状态失效、连接测试与显式录制控制组件测试
│   │   ├── ObsPanel.tsx  # OBS 本机连接设置、密码保存与连接测试、场景和录制面板
│   │   └── index.ts  # OBS 功能模块的公开组件出口
│   ├── training/  # 仅音频与可选文本训练、转写校对、任务版本和离线测量界面
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── TrainingPanel.audio.test.tsx  # 训练片段多格式转换上传、异步重选、容量与导入失败回归测试
│   │   ├── TrainingPanel.feedback.test.tsx  # 验证训练测量行内结果、异常弹窗、模型中断提示与终态竞态去重
│   │   ├── TrainingPanel.preferences.test.tsx  # 验证训练偏好恢复、服务隔离和存储异常回退
│   │   ├── TrainingPanel.saved.test.tsx  # 音色确认后资源同步、保存重开切换、删除与失败重试测试
│   │   ├── TrainingPanel.test.tsx  # 训练配置独立选择、服务忙碌时编辑、提交审核与试听取消交互测试
│   │   ├── TrainingPanel.transcription.test.tsx  # 训练声音与文本模式、空白转写行内结果及错误弹窗、审核与过期请求回归测试
│   │   ├── TrainingPanel.tsx  # 本地训练偏好、任务、音色版本、模型开关与资源变更通知工作区
│   │   ├── TrainingPanel.workspace.test.tsx  # 训练页签与素材分页、音色归组、性能参数及独立模型启停测试
│   │   ├── TrainingResultDialog.tsx  # 训练成功行内提示、错误弹窗、建议提示及键盘焦点恢复
│   │   ├── TrainingVoiceLibrary.tsx  # 按参考音色归组并经用户确认后切换的训练音色库、版本选择与单版本操作
│   │   ├── index.ts  # 训练面板公开组件导出
│   │   ├── trainingFeedback.ts  # 训练操作结果、任务终态通知及失败原因对应的改正建议
│   │   ├── useTraining.test.tsx  # 训练轮询独立更新、故障恢复与取消迟到结果测试
│   │   └── useTraining.ts  # 训练状态和资源轮询、版本选用通知、结果反馈与异步取消保护
│   ├── viewers/  # 管理员只读观众档案与持久事件查询页面
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── MemoryCard.tsx  # 记忆证据、版本及纠正删除冻结操作卡片
│   │   ├── RelationshipPanel.test.tsx  # 关系管理与图降级显示及请求幂等测试
│   │   ├── RelationshipPanel.tsx  # 关系图、来源证据和确认撤销重建管理面板
│   │   ├── ViewerDetail.test.tsx  # 观众管理详情、幂等重试和切换状态回归测试
│   │   ├── ViewerDetail.tsx  # 管理员陪伴账本、礼物和记忆详情面板
│   │   ├── ViewerMergePanel.test.tsx  # 身份合并显式确认与陈旧预览隔离测试
│   │   ├── ViewerMergePanel.tsx  # 身份合并预览、风险确认和目标切换面板
│   │   ├── ViewerPanel.test.tsx  # 观众与事件页面加载、分页及错误反馈测试
│   │   ├── ViewerPanel.tsx  # 可展开收起并在框内滚动的观众与事件面板，分页展示身份、昵称历史、持久事件与未确认接收缺口
│   │   └── index.ts  # 观众查询功能组件的公共导出
│   ├── voices/  # 参考素材、音色试听和训练任务界面
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── VoicePanel.test.tsx  # 多格式音色上传选择试听、删除确认失败及空状态交互测试
│   │   ├── VoicePanel.transcription.test.tsx  # 参考音频自动回填、同源音频文本上传及转写竞态回归测试
│   │   ├── VoicePanel.tsx  # 音色导入校验、参考文本自动转写、选择试听和删除管理及训练入口
│   │   ├── index.ts  # 音色管理：参考素材、试听与训练任务展示；不在浏览器执行模型推理。
│   │   ├── types.ts  # 音色列表选择试听、上传删除及文件清理重试能力契约
│   │   └── useReferenceTranscription.ts  # 参考音频本地自动转写、手工文本保护及过期请求取消
│   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── services/  # 前端访问主服务、Linux 启动管理与 Windows 桌面能力的统一边界
│   ├── audio/  # 参考音频与训练片段的浏览器解码、格式转换和音频校验
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── index.test.ts  # 多格式音频转换、PCM 编码、文件大小、时长与解码失败测试
│   │   ├── index.ts  # MP3 等多格式音频导入、离线解码与兼容 PCM16 WAV 转换
│   │   ├── wav.test.ts  # 参考与训练音频的 WAV 格式、时长、容量与静音边界测试
│   │   └── wav.ts  # 参考音频与训练片段的 PCM16 WAV 结构及有效性校验
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
│   │   ├── agent-observability.test.ts  # Agent 观察查询 URL、严格响应、安全来源、错误和取消测试
│   │   ├── agent-observability.ts  # 带认证和严格运行时校验的 Agent 调度及 Trace 查询客户端
│   │   ├── agent.bounds.test.ts  # Agent 合法最大快照、冷却与礼物数量边界回归测试
│   │   ├── agent.failures.test.ts  # Agent 畸形响应、HTTP 错误、超时与取消测试
│   │   ├── agent.interaction.test.ts  # Agent 互动配置及 SC、进房事件响应校验测试
│   │   ├── agent.requests.test.ts  # Agent 查询、设置、暂停恢复与事件批量请求契约测试
│   │   ├── agent.ts  # 独立 Agent HTTP 客户端、超时取消与运行时响应校验
│   │   ├── auth.test.ts  # 会话登录撤销、来源隔离及并发请求回归测试
│   │   ├── auth.ts  # 按主服务来源保存在内存的管理员会话与认证请求封装
│   │   ├── companionship.test.ts  # 陪伴管理服务契约与无效响应测试
│   │   ├── companionship.ts  # 陪伴详情、积分调整撤销和礼物确认请求
│   │   ├── eventPayload.ts  # 聊天、礼物、SC 与进房事件载荷共用校验
│   │   ├── failures.test.ts  # HTTP 错误、协议校验、取消和超时测试
│   │   ├── index.ts  # 带超时和取消的主服务 HTTP 客户端
│   │   ├── live.failures.test.ts  # 直播请求异常、超时、取消及脱敏配置快照校验测试
│   │   ├── live.requests.test.ts  # 直播控制和凭据配置请求、方法与载荷测试
│   │   ├── live.ts  # 直播状态与控制、面板凭据设置读写及响应校验客户端
│   │   ├── llm-runtime.test.ts  # 运行服务路由、契约、错误脱敏和超时取消测试
│   │   ├── llm-runtime.ts  # 运行设置、可关联 Trace 的用量和活动 HTTP 客户端及严格响应验证
│   │   ├── llm.reasoning.test.ts  # 推理预览 HTTP 契约、档位和预算响应校验测试
│   │   ├── llm.test.ts  # LLM 配置、模型目录及连接测试客户端的契约与异常测试
│   │   ├── llm.ts  # LLM 配置、模型列表与推理预览的认证请求、契约校验及取消处理
│   │   ├── memory.test.ts  # 记忆管理请求、版本和响应边界测试
│   │   ├── memory.ts  # 记忆查询及有条件管理操作服务
│   │   ├── obs.test.ts  # OBS 控制和设置请求、地址响应校验、失败超时及不重放请求测试
│   │   ├── obs.ts  # OBS 控制和脱敏连接设置读写的有界 HTTP 客户端
│   │   ├── relationships.test.ts  # 关系服务请求参数和响应边界测试
│   │   ├── relationships.ts  # 关系事实操作及图任务状态重建服务
│   │   ├── requests.test.ts  # HTTP 请求与成功响应测试
│   │   ├── resources.failures.test.ts  # 资源响应边界、请求失败、超时与取消测试
│   │   ├── resources.requests.test.ts  # 资源增删接口负载、空绑定及安装模型响应关联测试
│   │   ├── resources.ts  # 资源 HTTP 与桌面操作客户端及运行时响应校验
│   │   ├── responses.ts  # 生成契约的运行时响应校验与错误映射
│   │   ├── training.test.ts  # 训练与转写契约、请求校验、保存删除及取消超时测试
│   │   ├── training.ts  # 训练性能、任务版本、模型启停与离线测量客户端及响应校验
│   │   ├── viewerMerge.test.ts  # 合并预览和应用请求契约测试
│   │   ├── viewerMerge.ts  # 身份合并预览与确认服务请求
│   │   ├── viewers.test.ts  # 观众查询地址、分页边界与服务错误测试
│   │   └── viewers.ts  # 管理员观众与持久事件分页 HTTP 客户端
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   └── trainingPreferences.ts  # 按服务地址读写并校验训练面板本地偏好
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
│   ├── live-fixtures.ts  # 直播连接状态、脱敏配置快照与路由响应测试夹具
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

<!-- directory-tree-sha256: bbb9e679e1ec2926c4a878d2c5697cb57d519d6d6724caf4f00ae7f4dedecdb0 -->
