# apps 目录索引

可执行应用入口与依赖组装

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
apps/  # 可执行应用入口与依赖组装
├── desktop/  # React 控制面板及 Windows 桌面外壳
│   ├── src/  # 按应用组装、业务功能、外部服务和公共能力组织的前端源码
│   │   ├── app/  # React 根页面组装与全局样式
│   │   │   ├── feedback/  # 全局操作结果弹窗、去重与面板反馈回归测试
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── OperationFeedback.test.tsx  # 行内结果隔离、错误队列焦点恢复、后台故障去重及启动状态回归测试
│   │   │   │   ├── OperationFeedback.tsx  # 分页面行内反馈、错误去重队列、无障碍错误弹窗与焦点恢复
│   │   │   │   ├── PanelFeedback.test.tsx  # 各控制面板成功行内反馈、业务失败、异步状态与输入校验弹窗测试
│   │   │   │   └── feedback.css  # 页面行内结果文本及错误弹窗的配色、遮罩与响应式样式
│   │   │   ├── guide/  # 控制面板集中使用指南与各功能操作步骤
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── GuidePanel.tsx  # 独立新手指南、可展开的功能用法与工作区跳转入口
│   │   │   │   └── content.ts  # 启动、语音、角色、互动、观众、直播和训练的中文使用步骤
│   │   │   ├── resources/  # 角色与音色功能的页面组装和共享状态
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── ResourcesPanel.lifecycle.test.tsx  # 资源读取与操作的卸载取消测试
│   │   │   │   ├── ResourcesPanel.preview.test.tsx  # 音色试听状态跟踪、执行端断线、失败与重试的前端回归测试
│   │   │   │   ├── ResourcesPanel.regressions.test.tsx  # 当前角色重新加载与安装成功后刷新失败回归
│   │   │   │   ├── ResourcesPanel.tsx  # 组装角色与音色面板并显示资源操作状态
│   │   │   │   ├── index.ts  # 资源管理页面公共入口
│   │   │   │   └── useResourcesController.ts  # 角色音色快照、资源删除及桌面模型管理共享控制器
│   │   │   ├── AdminGate.test.tsx  # 认证启停、登录退出、错误重试与过期响应竞态回归测试
│   │   │   ├── AdminGate.tsx  # 默认直接访问控制面板及显式认证部署的会话检查、登录退出与访问门禁
│   │   │   ├── App.desktop.test.tsx  # 原生桌面地址初始化、导航功能请求地址与失败回归
│   │   │   ├── App.launcher.test.tsx  # 服务启停后的业务门控、环境页面常驻访问与模型检索草稿保留测试
│   │   │   ├── App.navigation.test.tsx  # 切页保留在途播报、历史深链接与未知页面回退的集成测试
│   │   │   ├── App.test.tsx  # 导航功能显隐、草稿保留与当前页标识的集成测试
│   │   │   ├── App.tsx  # 组装客户端与原生配置，选择受管或手动模式的导航控制台
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── ManagedWorkspace.tsx  # 受管服务状态、模型启用提示与功能工作区集成
│   │   │   ├── Workspace.guide.test.tsx  # 服务未就绪时访问指南、指南跳转与草稿保留的回归测试
│   │   │   ├── Workspace.tsx  # 简洁侧栏、独立环境与指南页面、快捷入口及稳定挂载的导航布局
│   │   │   ├── WorkspaceIcon.tsx  # 控制台导航、品牌猫形与快捷操作的代码内 SVG 图标
│   │   │   ├── navigation.ts  # 控制台功能导航、分组、页面说明与 URL fragment 映射
│   │   │   ├── styles.css  # 控制台公共样式、响应式布局与服务滑动开关样式
│   │   │   ├── useWorkspaceNavigation.ts  # 页面选择、访问记录与浏览器前进后退同步
│   │   │   └── workspace.css  # 柔和工作区主题、统一控件、响应式布局与训练、使用指南和连接配置样式
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
│   │   │   │   ├── CharacterPanel.test.tsx  # 角色与安装模型删除确认、导入保存加载及能力预览交互测试
│   │   │   │   ├── CharacterPanel.tsx  # 角色配置及安装模型增删、音色绑定、口型和热键管理界面
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── index.ts  # 角色管理：VTS 模型选择、表情动作映射及导入操作的界面。
│   │   │   │   └── types.ts  # 角色档案与本机模型增删管理界面的状态和能力契约
│   │   │   ├── connections/  # 直播平台凭据配置、连接控制与运行状态展示
│   │   │   │   ├── ConnectionPanel.test.tsx  # 直播连接面板状态展示、按钮规则与错误交互测试
│   │   │   │   ├── ConnectionPanel.tsx  # 直播凭据配置入口、连接控制、运行计数与错误反馈面板
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── LiveSettingsForm.test.tsx  # 直播配置读写、凭据保留清除、校验及连接联动测试
│   │   │   │   ├── LiveSettingsForm.tsx  # 哔哩哔哩直播凭据配置表单、已保存提示和保存反馈
│   │   │   │   ├── index.ts  # 直播平台连接功能公共出口
│   │   │   │   ├── polling.test.tsx  # 直播连接轮询串行、严格模式、迟到响应及卸载取消测试
│   │   │   │   └── useConnectionController.ts  # 直播连接轮询、操作互斥、取消及响应顺序控制器
│   │   │   ├── launcher/  # 控制面板服务开关、启动状态及新人引导
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── LauncherControls.tsx  # 主服务、TTS 和 Windows 执行端三开关、状态及本机配置位置
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
│   │   │   ├── llm/  # LLM 接入配置功能
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── LlmPanel.test.tsx  # LLM 面板草稿、密钥与生命周期测试
│   │   │   │   ├── LlmPanel.tsx  # LLM 服务商、协议、模型、密钥与连接测试面板
│   │   │   │   └── index.ts  # LLM 功能公共入口
│   │   │   ├── model-library/  # 环境检查、本地语音模型选择与官方模型下载管理界面
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── ModelLibraryPanel.test.tsx  # 环境门控、模型选择、分页检索、会话失效和下载取消交互测试
│   │   │   │   ├── ModelLibraryPanel.tsx  # 独立环境检测、分页模型库、下载进度与手动启动入口
│   │   │   │   └── useModelLibrary.ts  # 模型状态轮询与串行操作控制，防止过期响应覆盖和重复提交
│   │   │   ├── obs/  # OBS 场景与录制控制面板
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── ObsPanel.test.tsx  # OBS 设置读写、密码清除、旧状态失效、连接测试与显式录制控制组件测试
│   │   │   │   ├── ObsPanel.tsx  # OBS 本机连接设置、密码保存与连接测试、场景和录制面板
│   │   │   │   └── index.ts  # OBS 功能模块的公开组件出口
│   │   │   ├── training/  # 仅音频与可选文本训练、转写校对、任务版本和离线测量界面
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── TrainingPanel.audio.test.tsx  # 训练片段多格式转换上传、异步重选、容量与导入失败回归测试
│   │   │   │   ├── TrainingPanel.feedback.test.tsx  # 验证训练测量行内结果、异常弹窗、模型中断提示与终态竞态去重
│   │   │   │   ├── TrainingPanel.preferences.test.tsx  # 验证训练偏好恢复、服务隔离和存储异常回退
│   │   │   │   ├── TrainingPanel.saved.test.tsx  # 音色保存重开切换、删除确认及清理失败重试交互测试
│   │   │   │   ├── TrainingPanel.test.tsx  # 训练配置独立选择、服务忙碌时编辑、提交审核与试听取消交互测试
│   │   │   │   ├── TrainingPanel.transcription.test.tsx  # 训练声音与文本模式、空白转写行内结果及错误弹窗、审核与过期请求回归测试
│   │   │   │   ├── TrainingPanel.tsx  # 本地训练偏好、性能设置、记录分页、音色版本、模型开关及结果弹窗工作区
│   │   │   │   ├── TrainingPanel.workspace.test.tsx  # 训练页签与素材分页、音色归组、性能参数及独立模型启停测试
│   │   │   │   ├── TrainingResultDialog.tsx  # 训练成功行内提示、错误弹窗、建议提示及键盘焦点恢复
│   │   │   │   ├── TrainingVoiceLibrary.tsx  # 按参考音色归组的训练音色库、版本选择与单版本操作
│   │   │   │   ├── index.ts  # 训练面板公开组件导出
│   │   │   │   ├── trainingFeedback.ts  # 训练操作结果、任务终态通知及失败原因对应的改正建议
│   │   │   │   ├── useTraining.test.tsx  # 训练轮询独立更新、故障恢复与取消迟到结果测试
│   │   │   │   └── useTraining.ts  # 训练资源与模型状态轮询、操作反馈、终态通知及异步取消保护
│   │   │   ├── viewers/  # 管理员只读观众档案与持久事件查询页面
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── MemoryCard.tsx  # 记忆证据、版本及纠正删除冻结操作卡片
│   │   │   │   ├── RelationshipPanel.test.tsx  # 关系管理与图降级显示及请求幂等测试
│   │   │   │   ├── RelationshipPanel.tsx  # 关系图、来源证据和确认撤销重建管理面板
│   │   │   │   ├── ViewerDetail.test.tsx  # 观众管理详情、幂等重试和切换状态回归测试
│   │   │   │   ├── ViewerDetail.tsx  # 管理员陪伴账本、礼物和记忆详情面板
│   │   │   │   ├── ViewerMergePanel.test.tsx  # 身份合并显式确认与陈旧预览隔离测试
│   │   │   │   ├── ViewerMergePanel.tsx  # 身份合并预览、风险确认和目标切换面板
│   │   │   │   ├── ViewerPanel.test.tsx  # 观众与事件页面加载、分页及错误反馈测试
│   │   │   │   ├── ViewerPanel.tsx  # 可展开收起并在框内滚动的观众与事件面板，分页展示身份、昵称历史、持久事件与未确认接收缺口
│   │   │   │   └── index.ts  # 观众查询功能组件的公共导出
│   │   │   ├── voices/  # 参考素材、音色试听和训练任务界面
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── VoicePanel.test.tsx  # 多格式音色上传选择试听、删除确认失败及空状态交互测试
│   │   │   │   ├── VoicePanel.tsx  # 音色导入校验、选择试听和删除管理及训练音色入口
│   │   │   │   ├── index.ts  # 音色管理：参考素材、试听与训练任务展示；不在浏览器执行模型推理。
│   │   │   │   └── types.ts  # 音色列表选择试听、上传删除及文件清理重试能力契约
│   │   │   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── services/  # 前端访问主服务、Linux 启动管理与 Windows 桌面能力的统一边界
│   │   │   ├── audio/  # 参考音频与训练片段的浏览器解码、格式转换和音频校验
│   │   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   │   ├── index.test.ts  # 多格式音频转换、PCM 编码、文件大小、时长与解码失败测试
│   │   │   │   ├── index.ts  # MP3 等多格式音频导入、离线解码与兼容 PCM16 WAV 转换
│   │   │   │   ├── wav.test.ts  # 参考与训练音频的 WAV 格式、时长、容量与静音边界测试
│   │   │   │   └── wav.ts  # 参考音频与训练片段的 PCM16 WAV 结构及有效性校验
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
│   │   │   │   ├── auth.test.ts  # 会话登录撤销、来源隔离及并发请求回归测试
│   │   │   │   ├── auth.ts  # 按主服务来源保存在内存的管理员会话与认证请求封装
│   │   │   │   ├── companionship.test.ts  # 陪伴管理服务契约与无效响应测试
│   │   │   │   ├── companionship.ts  # 陪伴详情、积分调整撤销和礼物确认请求
│   │   │   │   ├── failures.test.ts  # HTTP 错误、协议校验、取消和超时测试
│   │   │   │   ├── index.ts  # 带超时和取消的主服务 HTTP 客户端
│   │   │   │   ├── live.failures.test.ts  # 直播请求异常、超时、取消及脱敏配置快照校验测试
│   │   │   │   ├── live.requests.test.ts  # 直播控制和凭据配置请求、方法与载荷测试
│   │   │   │   ├── live.ts  # 直播状态与控制、面板凭据设置读写及响应校验客户端
│   │   │   │   ├── llm.test.ts  # LLM 客户端路由、校验、错误与超时测试
│   │   │   │   ├── llm.ts  # LLM 设置与连接测试 HTTP 客户端及响应校验
│   │   │   │   ├── memory.test.ts  # 记忆管理请求、版本和响应边界测试
│   │   │   │   ├── memory.ts  # 记忆查询及有条件管理操作服务
│   │   │   │   ├── obs.test.ts  # OBS 控制和设置请求、地址响应校验、失败超时及不重放请求测试
│   │   │   │   ├── obs.ts  # OBS 控制和脱敏连接设置读写的有界 HTTP 客户端
│   │   │   │   ├── relationships.test.ts  # 关系服务请求参数和响应边界测试
│   │   │   │   ├── relationships.ts  # 关系事实操作及图任务状态重建服务
│   │   │   │   ├── requests.test.ts  # HTTP 请求与成功响应测试
│   │   │   │   ├── resources.failures.test.ts  # 资源响应边界、请求失败、超时与取消测试
│   │   │   │   ├── resources.requests.test.ts  # 资源增删接口负载、空绑定及安装模型响应关联测试
│   │   │   │   ├── resources.ts  # 资源 HTTP 与桌面操作客户端及运行时响应校验
│   │   │   │   ├── responses.ts  # 生成契约的运行时响应校验与错误映射
│   │   │   │   ├── training.test.ts  # 训练与转写契约、请求校验、保存删除及取消超时测试
│   │   │   │   ├── training.ts  # 训练性能、任务版本、模型启停与离线测量客户端及响应校验
│   │   │   │   ├── viewerMerge.test.ts  # 合并预览和应用请求契约测试
│   │   │   │   ├── viewerMerge.ts  # 身份合并预览与确认服务请求
│   │   │   │   ├── viewers.test.ts  # 观众查询地址、分页边界与服务错误测试
│   │   │   │   └── viewers.ts  # 管理员观众与持久事件分页 HTTP 客户端
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── trainingPreferences.ts  # 按服务地址读写并校验训练面板本地偏好
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
│   │   │   ├── live-fixtures.ts  # 直播连接状态、脱敏配置快照与路由响应测试夹具
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
│   │   │   ├── state.rs  # Agent 查询控制、本机设置保存、原子事件接收及语音状态同步
│   │   │   └── worker.rs  # 实时 Agent 调度、资料期限版本栅栏及持久回应关联
│   │   ├── config/  # 按 Agent 与模型能力拆分的配置校验
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── agent.rs  # Agent 人设和有界调度参数的 TOML 配置
│   │   │   ├── auth.rs  # 管理员和设备私有凭据来源与会话期限配置校验
│   │   │   ├── graph.rs  # 图副本连接和私有凭据来源配置
│   │   │   ├── live.rs  # 直播接入开关、兼容环境变量与本机覆盖凭据的校验和脱敏
│   │   │   ├── llm.rs  # LLM 提供商协议、地址、模型及资源限额校验与密钥脱敏
│   │   │   ├── memory.rs  # 独立提取嵌入模型与请求预算配置
│   │   │   ├── resources.rs  # Linux 资源保存目录和引擎共享挂载配置
│   │   │   ├── training.rs  # 训练路径、本地识别模型、超时及受管推理默认权重配置校验
│   │   │   └── viewers.rs  # 默认开启的观众与记忆持久化、角色范围和数据库环境变量配置校验
│   │   ├── live/  # 官方直播源组装及异步连接、接收、清理与重连驱动
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── bootstrap.rs  # 使用面板本机凭据或兼容环境变量组装官方直播源
│   │   │   └── worker.rs  # 直播事件接收、去重入队、取消、会话清理和退避重连
│   │   ├── transport/  # 控制接口、跨端连接与协议到领域对象的转换
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   ├── agent.rs  # Agent 状态控制与批量事件输入的 HTTP 边界
│   │   │   ├── auth.rs  # 默认软件管理者访问与可选管理员会话、HTTP 和 WebSocket 角色认证
│   │   │   ├── bridge.rs  # 执行端双通道桥接、上下文版本校验及完成回执持久认可
│   │   │   ├── companionship.rs  # 受认证保护的陪伴账本和礼物管理接口
│   │   │   ├── error.rs  # 稳定的结构化 HTTP 错误映射
│   │   │   ├── http.rs  # HTTP 路由、最小健康接口及管理员与来源边界组装
│   │   │   ├── live.rs  # 直播连接控制及脱敏设置查询和本机保存 HTTP 入口
│   │   │   ├── llm.rs  # LLM 接入配置读写与草稿连接测试 HTTP 入口
│   │   │   ├── mapping.rs  # protocol DTO 与 domain 类型的显式转换，避免序列化字段影响领域规则。
│   │   │   ├── memory.rs  # 记忆纠正删除冻结及任务恢复管理接口
│   │   │   ├── mod.rs  # HTTP / WebSocket 输入与输出适配；在协议 DTO 与领域对象之间进行映射。
│   │   │   ├── obs.rs  # 通过执行端代理 OBS 控制、脱敏设置查询及本机配置保存入口
│   │   │   ├── origin.rs  # 浏览器请求来源校验及 HTTP 来源中间件
│   │   │   ├── relationships.rs  # 关系证据管理和图状态重建接口
│   │   │   ├── resources.rs  # 音色和角色增删选择、模型删除代理及能力验证路由
│   │   │   ├── runtime.rs  # 离线推理测量、GPU 采样错误传播与预设查询
│   │   │   ├── training.rs  # 训练素材导入、性能设置、按所选 GPU 准入及版本试听保存接口
│   │   │   ├── training_models.rs  # 模型内存状态查询及带资源互斥和取消保护的启停接口
│   │   │   ├── viewer_merge.rs  # 身份合并预览及显式确认管理接口
│   │   │   ├── viewers.rs  # 控制面板的有界观众和持久事件摘要查询
│   │   │   └── websocket.rs  # 唯一执行端连接准入及音频配对校验
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── agent.rs  # Agent 异步驱动与服务状态组装入口
│   │   ├── agent_settings.rs  # 本机 Agent 人设话题与互动偏好的校验加载及原子保存
│   │   ├── auth.rs  # 管理员短期会话、凭据摘要、撤销及独立设备授权
│   │   ├── bootstrap.rs  # 配置与持久能力组装、模型适配器及后台任务生命周期
│   │   ├── companionship.rs  # 设备完成回执的有界异步账本提交与失败诊断
│   │   ├── config.rs  # TOML 配置、本机 LLM 与 Agent 覆盖加载及启动前全局校验
│   │   ├── gpu.rs  # 训练试听测量独占租约、保留直播及 Agent 状态的音色切换准入与互斥测试
│   │   ├── graph.rs  # 图同步恢复及 SQL 权威事实降级查询
│   │   ├── lib.rs  # 可注入适配器的服务模块导出与集成测试入口
│   │   ├── live.rs  # 直播连接单会话状态所有权、非阻塞控制与面板配置即时应用
│   │   ├── live_settings.rs  # 直播凭据的本机原子保存、重启加载和脱敏配置快照
│   │   ├── llm_settings.rs  # 本机 LLM 覆盖配置的原子保存、凭据隔离与重启状态查询
│   │   ├── main.rs  # Linux / WSL 主服务入口。业务编排位于 meowlive-application。
│   │   ├── memory.rs  # 记忆后台任务、上下文检索及资料失效控制
│   │   ├── memory_worker_tests.rs  # 真实 HTTP 模型与 PostgreSQL 工作队列及过期观察器联调测试
│   │   ├── resources.rs  # 桌面资源请求关联、单操作准入与取消生命周期
│   │   ├── state.rs  # 语音、Agent、唯一执行桥接和直播连接的共享状态及取消生命周期
│   │   ├── viewers.rs  # 持久事件接收与模拟来源隔离、回应调度和缺口诊断
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
│   │   │   └── mod.rs  # 主服务测试子进程启动重启清理和受控 WAV 构造
│   │   ├── support/  # 主服务集成测试公共夹具
│   │   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   │   └── mod.rs  # 受控合成器、HTTP 请求与双 WebSocket 连接夹具
│   │   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   │   ├── admin_auth.rs  # 管理员登录撤销、会话过期、设备权限隔离和来源边界测试
│   │   ├── agent_api.rs  # Agent 默认暂停、设置、事件校验、去重与停止接口测试
│   │   ├── agent_bootstrap.rs  # LLM 启动配置、缺失环境变量及无认证本地模型测试
│   │   ├── agent_cancellation.rs  # 暂停停止配置修改和断线取消在途 LLM 的集成测试
│   │   ├── agent_capacity.rs  # 事件批量请求大小和容量拒绝的原子性测试
│   │   ├── agent_configuration.rs  # Agent 和 LLM 的默认配置、字段边界及解析错误脱敏测试
│   │   ├── agent_process.rs  # 真实主服务重启恢复 Agent 设置及受控模型静音播报闭环测试
│   │   ├── agent_receipt_retention.rs  # 语音历史裁剪时保留 Agent 已完成播放结果的回归测试
│   │   ├── agent_retries.rs  # 模型临时错误分类和有限重试次数的集成测试
│   │   ├── agent_runtime.rs  # 模拟事件单次回复到设备播放完成、冷却后不重播及断线未知状态集成测试
│   │   ├── agent_settings.rs  # Agent 设置持久保存、配置隔离、并发一致性与失败保留测试
│   │   ├── bridge_handshake.rs  # 协议版本、唯一执行端与双连接配对测试
│   │   ├── configuration.rs  # 示例配置兼容及无效参数拒绝测试
│   │   ├── http_api.rs  # 状态、播报输入与停止接口测试
│   │   ├── knowledge_configuration.rs  # 记忆与图配置启用条件和独立模型参数测试
│   │   ├── live_admission.rs  # 直播事件容量丢弃、无效事件及退出清理测试
│   │   ├── live_cleanup.rs  # 直播延迟清理、清理失败、房间状态与退避取消回归测试
│   │   ├── live_configuration.rs  # 直播凭据缺失、环境变量边界与配置验证测试
│   │   ├── live_http.rs  # 直播默认禁用状态、连接入口和配置范围测试
│   │   ├── live_lifecycle.rs  # 单直播连接、取消迟到结果、事件去重与重连终止测试
│   │   ├── live_process.rs  # 真实主服务进程的直播凭据组装、默认不连接与公开响应脱敏测试
│   │   ├── live_runtime.rs  # 受控平台礼物与重复帧经真实适配器、Agent、语音和静音设备完成回执的联调测试
│   │   ├── live_settings.rs  # 直播面板配置持久化、热生效、凭据保留清除、请求校验与真实重启测试
│   │   ├── llm_profile.rs  # LLM 协议组装、私有配置持久化、重启加载及 HTTP 验证
│   │   ├── m5_config.rs  # 本地预设地址资源上限及训练配置约束测试
│   │   ├── obs_http.rs  # OBS 控制及私有设置 HTTP 参数、执行端桥接和脱敏结果测试
│   │   ├── request_origin.rs  # HTTP 来源拒绝和无副作用保障测试
│   │   ├── resources_bridge.rs  # 桌面资源关联、模型删除引用与并发保护、断连响应边界测试
│   │   ├── resources_characters.rs  # 角色加载确认、预览验证及映射变更失效测试
│   │   ├── resources_http.rs  # 初始空音色、角色删除、非法资源及音色别名 HTTP 测试
│   │   ├── resources_process.rs  # 真实服务进程上传恢复试听、Agent、资源删除持久化链路测试
│   │   ├── resources_runtime.rs  # 面板经真实桌面执行库到受控 VTS 的角色、热键与停止链路测试
│   │   ├── runtime_loop.rs  # 真实服务与独立桌面运行时的静音播放集成测试
│   │   ├── speech_delivery.rs  # PCM 下发、设备回执门控及独立停止通道测试
│   │   ├── synthesis_cancellation.rs  # 合成停止、断线未知与重连不重播集成测试
│   │   ├── training_http.rs  # 训练配置、音色版本删除与当前选择清理、存储故障的 HTTP 测试
│   │   ├── training_models.rs  # 受控 HTTP 验证模型开关及无需开启训练的默认推理流程
│   │   ├── training_process.rs  # 真实主服务训练上传、音色保存重启、版本试听启用、取消及 SIGTERM 子树清理测试
│   │   ├── viewer_configuration.rs  # 默认存储与免登录管理、角色范围和数据库配置约束测试
│   │   ├── viewer_events.rs  # 数据库失败与满队列接收、UTC 保存和重启去重的服务测试
│   │   ├── viewer_knowledge_runtime.rs  # 真实数据库与模拟模型执行端的资料失效联调测试
│   │   └── viewer_process.rs  # 默认免登录及显式认证下的持久观众事件查询、记忆接口与重启去重进程测试
│   ├── Cargo.toml  # 该 Rust 包的名称、workspace 配置与模块依赖声明
│   └── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
└── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
```

可继续查看各子目录的索引：

- [desktop/](desktop/DIRECTORY.md)：React 控制面板及 Windows 桌面外壳
- [server/](server/DIRECTORY.md)：Linux / WSL 主服务入口、配置和传输边界

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 8b5fbb0d425bad1ccedf24bcbcfbbfe2be3ff95a5eb5ff85d13f49c704983a91 -->
