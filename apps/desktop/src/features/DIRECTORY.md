# features 目录索引

面向用户的功能模块，各自封装组件与状态

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
features/  # 面向用户的功能模块，各自封装组件与状态
├── agent/  # Agent 人设、话题和互动策略设置
│   ├── AgentPanel.test.tsx  # Agent 状态控制、错误呈现、轮询竞态与取消清理测试
│   ├── AgentPanel.tsx  # Agent 运行条件、暂停恢复、设置、事件输入与历史的组合面板
│   ├── AgentSettingsForm.bounds.test.tsx  # 默认空话题及人设话题长度与后端一致性的回归测试
│   ├── AgentSettingsForm.interaction.test.tsx  # 互动策略表单持久化、草稿保留与阈值边界测试
│   ├── AgentSettingsForm.test.tsx  # Agent 设置草稿、输入校验与毫秒请求映射测试
│   ├── AgentSettingsForm.tsx  # 保留草稿并编辑人设、冷却和弹幕欢迎互动策略的 Agent 设置表单
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── EventHistory.test.tsx  # Agent 事件内容、状态、播报关联与错误展示测试
│   ├── EventHistory.tsx  # Agent 事件状态、关联播报及错误历史列表
│   ├── EventSimulator.test.tsx  # 模拟事件校验、成功清空、失败保留与重复反馈测试
│   ├── EventSimulator.tsx  # 聊天、礼物、SC 与进房模拟及 JSON 批量回放表单
│   ├── index.ts  # Agent 自动互动面板的功能出口
│   └── useAgentController.ts  # 修订号防回滚及卸载取消的串行 Agent 轮询与动作控制器
├── characters/  # 角色模型选择、导入与动作映射界面
│   ├── CharacterPanel.test.tsx  # 角色与安装模型删除确认、导入保存加载及能力预览交互测试
│   ├── CharacterPanel.tsx  # 角色配置及安装模型增删、音色绑定、口型和热键管理界面
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── index.ts  # 角色管理：VTS 模型选择、表情动作映射及导入操作的界面。
│   └── types.ts  # 角色档案与本机模型增删管理界面的状态和能力契约
├── connections/  # 直播平台凭据配置、连接控制与运行状态展示
│   ├── ConnectionPanel.test.tsx  # 直播连接面板状态展示、按钮规则与错误交互测试
│   ├── ConnectionPanel.tsx  # 直播凭据配置入口、连接控制、运行计数与错误反馈面板
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── LiveSettingsForm.test.tsx  # 直播配置读写、凭据保留清除、校验及连接联动测试
│   ├── LiveSettingsForm.tsx  # 哔哩哔哩直播凭据配置表单、已保存提示和保存反馈
│   ├── index.ts  # 直播平台连接功能公共出口
│   ├── polling.test.tsx  # 直播连接轮询串行、严格模式、迟到响应及卸载取消测试
│   └── useConnectionController.ts  # 直播连接轮询、操作互斥、取消及响应顺序控制器
├── launcher/  # 控制面板服务开关、启动状态及新人引导
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── LauncherControls.tsx  # 主服务、TTS 和 Windows 执行端三开关、状态及本机配置位置
│   ├── LauncherPanel.test.tsx  # 滑动开关真实状态、启动取消、外部服务与断线交互测试
│   ├── LauncherPanel.tsx  # 服务开关展示与业务就绪门控的可复用组合入口
│   ├── index.ts  # 服务开关展示、状态控制器与组合面板公共导出
│   └── useLauncher.ts  # 启动管理状态轮询和串行启停请求，防止过期状态覆盖
├── live/  # 直播工作台、弹幕观察与播报控制
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── SpeechPanel.scenarios.test.tsx  # 提交、停止、失败与重复操作场景测试
│   ├── SpeechPanel.test.tsx  # 输入校验、可用状态和历史展示组件测试
│   ├── SpeechPanel.tsx  # 中文人工语音播报控制面板
│   ├── index.ts  # 直播工作台：会话状态、弹幕观察、播放状态与人工控制。
│   ├── polling.test.tsx  # 状态刷新、取消清理与迟到响应场景测试
│   └── useSpeechController.ts  # 可取消的串行状态刷新与播报操作状态
├── llm/  # LLM 接入配置功能
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── LlmPanel.models.test.tsx  # LLM 模型发现、自动识别、选择与连接切换交互测试
│   ├── LlmPanel.reasoning.test.tsx  # 推理档位选择、映射展示、预算阻断与请求竞态回归测试
│   ├── LlmPanel.test.tsx  # LLM 面板草稿、密钥与生命周期测试
│   ├── LlmPanel.tsx  # LLM 连接配置、模型获取与选择、推理档位预览和测试保存面板
│   ├── index.ts  # LLM 功能公共入口
│   └── useReasoningPreview.ts  # 推理能力预览的防抖、取消、重试与迟到响应隔离
├── llm-runtime/  # 模型运行能力、用量费用与 Agent 实时活动界面
│   ├── AgentActivityPanel.test.tsx  # 活动更新和隐藏卸载取消轮询测试
│   ├── AgentActivityPanel.tsx  # Agent 阶段、工具进度与网页来源展示
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── LlmRuntimePanel.test.tsx  # 运行设置、密钥绑定、费用和取消交互测试
│   ├── LlmRuntimePanel.tsx  # 运行配置与用量折叠区入口
│   ├── RuntimeIntegration.test.tsx  # LLM 与 Agent 原页面运行能力接入测试
│   ├── RuntimeSettingsPanel.tsx  # 运行开关、搜索密钥及模型单价编辑与保存
│   ├── UsagePanel.tsx  # 按日期和模型查询已报告用量与估算费用
│   ├── llm-runtime.css  # 运行能力与用量界面的粉色响应式样式
│   ├── runtime-fixtures.ts  # 运行配置、用量及活动测试样例
│   └── useRuntimeVisibility.ts  # 按工作区和文档可见性暂停运行请求
├── model-library/  # 环境检查、本地语音模型选择与官方模型下载管理界面
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── InstalledModels.tsx  # 同页分组展示声音生成与语音识别模型、独立选择及下载引导
│   ├── ModelLibraryPanel.test.tsx  # 环境门控、模型选择、分页检索、会话失效和下载取消交互测试
│   ├── ModelLibraryPanel.tsx  # 环境检测、音色训练与转文本分类模型库、用途与接入状态筛选及独立分页下载
│   └── useModelLibrary.ts  # 模型状态轮询与串行操作控制，防止过期响应覆盖和重复提交
├── obs/  # OBS 场景与录制控制面板
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── ObsPanel.test.tsx  # OBS 设置读写、密码清除、旧状态失效、连接测试与显式录制控制组件测试
│   ├── ObsPanel.tsx  # OBS 本机连接设置、密码保存与连接测试、场景和录制面板
│   └── index.ts  # OBS 功能模块的公开组件出口
├── training/  # 仅音频与可选文本训练、转写校对、任务版本和离线测量界面
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── TrainingPanel.audio.test.tsx  # 训练片段多格式转换上传、异步重选、容量与导入失败回归测试
│   ├── TrainingPanel.feedback.test.tsx  # 验证训练测量行内结果、异常弹窗、模型中断提示与终态竞态去重
│   ├── TrainingPanel.preferences.test.tsx  # 验证训练偏好恢复、服务隔离和存储异常回退
│   ├── TrainingPanel.saved.test.tsx  # 音色保存重开切换、删除确认及清理失败重试交互测试
│   ├── TrainingPanel.test.tsx  # 训练配置独立选择、服务忙碌时编辑、提交审核与试听取消交互测试
│   ├── TrainingPanel.transcription.test.tsx  # 训练声音与文本模式、空白转写行内结果及错误弹窗、审核与过期请求回归测试
│   ├── TrainingPanel.tsx  # 本地训练偏好、性能设置、记录分页、音色版本、模型开关及结果弹窗工作区
│   ├── TrainingPanel.workspace.test.tsx  # 训练页签与素材分页、音色归组、性能参数及独立模型启停测试
│   ├── TrainingResultDialog.tsx  # 训练成功行内提示、错误弹窗、建议提示及键盘焦点恢复
│   ├── TrainingVoiceLibrary.tsx  # 按参考音色归组的训练音色库、版本选择与单版本操作
│   ├── index.ts  # 训练面板公开组件导出
│   ├── trainingFeedback.ts  # 训练操作结果、任务终态通知及失败原因对应的改正建议
│   ├── useTraining.test.tsx  # 训练轮询独立更新、故障恢复与取消迟到结果测试
│   └── useTraining.ts  # 训练资源与模型状态轮询、操作反馈、终态通知及异步取消保护
├── viewers/  # 管理员只读观众档案与持久事件查询页面
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── MemoryCard.tsx  # 记忆证据、版本及纠正删除冻结操作卡片
│   ├── RelationshipPanel.test.tsx  # 关系管理与图降级显示及请求幂等测试
│   ├── RelationshipPanel.tsx  # 关系图、来源证据和确认撤销重建管理面板
│   ├── ViewerDetail.test.tsx  # 观众管理详情、幂等重试和切换状态回归测试
│   ├── ViewerDetail.tsx  # 管理员陪伴账本、礼物和记忆详情面板
│   ├── ViewerMergePanel.test.tsx  # 身份合并显式确认与陈旧预览隔离测试
│   ├── ViewerMergePanel.tsx  # 身份合并预览、风险确认和目标切换面板
│   ├── ViewerPanel.test.tsx  # 观众与事件页面加载、分页及错误反馈测试
│   ├── ViewerPanel.tsx  # 可展开收起并在框内滚动的观众与事件面板，分页展示身份、昵称历史、持久事件与未确认接收缺口
│   └── index.ts  # 观众查询功能组件的公共导出
├── voices/  # 参考素材、音色试听和训练任务界面
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── VoicePanel.test.tsx  # 多格式音色上传选择试听、删除确认失败及空状态交互测试
│   ├── VoicePanel.transcription.test.tsx  # 参考音频自动回填、同源音频文本上传及转写竞态回归测试
│   ├── VoicePanel.tsx  # 音色导入校验、参考文本自动转写、选择试听和删除管理及训练入口
│   ├── index.ts  # 音色管理：参考素材、试听与训练任务展示；不在浏览器执行模型推理。
│   ├── types.ts  # 音色列表选择试听、上传删除及文件清理重试能力契约
│   └── useReferenceTranscription.ts  # 参考音频本地自动转写、手工文本保护及过期请求取消
└── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
```

可继续查看各子目录的索引：

- [agent/](agent/DIRECTORY.md)：Agent 人设、话题和互动策略设置
- [characters/](characters/DIRECTORY.md)：角色模型选择、导入与动作映射界面
- [connections/](connections/DIRECTORY.md)：直播平台凭据配置、连接控制与运行状态展示
- [launcher/](launcher/DIRECTORY.md)：控制面板服务开关、启动状态及新人引导
- [live/](live/DIRECTORY.md)：直播工作台、弹幕观察与播报控制
- [llm/](llm/DIRECTORY.md)：LLM 接入配置功能
- [llm-runtime/](llm-runtime/DIRECTORY.md)：模型运行能力、用量费用与 Agent 实时活动界面
- [model-library/](model-library/DIRECTORY.md)：环境检查、本地语音模型选择与官方模型下载管理界面
- [obs/](obs/DIRECTORY.md)：OBS 场景与录制控制面板
- [training/](training/DIRECTORY.md)：仅音频与可选文本训练、转写校对、任务版本和离线测量界面
- [viewers/](viewers/DIRECTORY.md)：管理员只读观众档案与持久事件查询页面
- [voices/](voices/DIRECTORY.md)：参考素材、音色试听和训练任务界面

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 74d3a3272895636cba80778fcecc691b819c38837c4143c385d2d95e195cb2df -->
