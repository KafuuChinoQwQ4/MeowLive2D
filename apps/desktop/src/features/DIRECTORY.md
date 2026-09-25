# features 目录索引

面向用户的功能模块，各自封装组件与状态

本文件由 `npm run tree:update` 生成，只列本目录的直接子目录；进入对应子目录查看下一层。每项右侧为大致用途。

```text
features/  # 面向用户的功能模块，各自封装组件与状态
├── agent/  # Agent 人设、系统提示词、话题和互动策略设置
├── agent-observability/  # Agent 调度原因、Trace 历史、模型 Turn 和执行时间线观察页面
├── characters/  # 角色模型选择、导入与动作映射界面
├── connections/  # 直播平台凭据配置、连接控制与运行状态展示
├── database-setup/  # 数据库官方来源、Windows 和源码安装启用指引
├── launcher/  # 控制面板服务开关、启动状态及新人引导
├── live/  # 直播工作台、弹幕观察与播报控制
├── llm/  # LLM 接入配置功能
├── llm-runtime/  # 模型运行能力、用量费用与 Agent 实时活动界面
├── model-library/  # 环境检查、本地语音模型选择与官方模型下载管理界面
├── obs/  # OBS 场景与录制控制面板
├── training/  # 仅音频与可选文本训练、转写校对、任务版本和离线测量界面
├── viewers/  # 管理员只读观众档案与持久事件查询页面
└── voices/  # 参考素材、音色试听和训练任务界面
```

可继续查看各子目录的索引：

- [agent/](agent/DIRECTORY.md)：Agent 人设、系统提示词、话题和互动策略设置
- [agent-observability/](agent-observability/DIRECTORY.md)：Agent 调度原因、Trace 历史、模型 Turn 和执行时间线观察页面
- [characters/](characters/DIRECTORY.md)：角色模型选择、导入与动作映射界面
- [connections/](connections/DIRECTORY.md)：直播平台凭据配置、连接控制与运行状态展示
- [database-setup/](database-setup/DIRECTORY.md)：数据库官方来源、Windows 和源码安装启用指引
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

文件内容变化会更新当前目录及祖先索引的指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 9250c41523cf7a27b0ce451c860e7c6d530364d39ebbe2ea1f663117e72b5eb6 -->
