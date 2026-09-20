# app 目录索引

React 根页面组装与全局样式

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
app/  # React 根页面组装与全局样式
├── feedback/  # 全局操作结果弹窗、去重与面板反馈回归测试
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── OperationFeedback.test.tsx  # 行内结果隔离、错误队列焦点恢复、后台故障去重及启动状态回归测试
│   ├── OperationFeedback.tsx  # 分页面行内反馈、错误去重队列、无障碍错误弹窗与焦点恢复
│   ├── PanelFeedback.test.tsx  # 各控制面板成功行内反馈、业务失败、异步状态与输入校验弹窗测试
│   └── feedback.css  # 页面行内结果文本及错误弹窗的配色、遮罩与响应式样式
├── guide/  # 控制面板集中使用指南与各功能操作步骤
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── GuidePanel.tsx  # 独立新手指南、可展开的功能用法与工作区跳转入口
│   └── content.ts  # 启动、语音、角色、互动、观众、直播和训练的中文使用步骤
├── resources/  # 角色与音色功能的页面组装和共享状态
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── ResourcesPanel.lifecycle.test.tsx  # 资源读取与操作的卸载取消测试
│   ├── ResourcesPanel.preview.test.tsx  # 音色试听状态跟踪、执行端断线、失败与重试的前端回归测试
│   ├── ResourcesPanel.regressions.test.tsx  # 当前角色重新加载与安装成功后刷新失败回归
│   ├── ResourcesPanel.tsx  # 组装角色与音色面板并显示资源操作状态
│   ├── index.ts  # 资源管理页面公共入口
│   └── useResourcesController.ts  # 角色音色快照、资源删除及桌面模型管理共享控制器
├── AdminGate.test.tsx  # 认证启停、登录退出、错误重试与过期响应竞态回归测试
├── AdminGate.tsx  # 默认直接访问控制面板及显式认证部署的会话检查、登录退出与访问门禁
├── App.desktop.test.tsx  # 原生桌面地址初始化、导航功能请求地址与失败回归
├── App.launcher.test.tsx  # 服务启停后的业务门控、环境页面常驻访问与模型检索草稿保留测试
├── App.navigation.test.tsx  # 切页保留在途播报、历史深链接与未知页面回退的集成测试
├── App.test.tsx  # 导航功能显隐、草稿保留与当前页标识的集成测试
├── App.tsx  # 组装客户端与原生配置，选择受管或手动模式的导航控制台
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── ManagedWorkspace.tsx  # 受管服务状态、模型启用提示与功能工作区集成
├── Workspace.guide.test.tsx  # 服务未就绪时访问指南、指南跳转与草稿保留的回归测试
├── Workspace.tsx  # 简洁侧栏、独立环境与指南页面、快捷入口及稳定挂载的导航布局
├── WorkspaceIcon.tsx  # 控制台导航、品牌猫形与快捷操作的代码内 SVG 图标
├── navigation.ts  # 控制台功能导航、分组、页面说明与 URL fragment 映射
├── styles.css  # 控制台公共样式、响应式布局与服务滑动开关样式
├── useWorkspaceNavigation.ts  # 页面选择、访问记录与浏览器前进后退同步
└── workspace.css  # 柔和工作区主题、统一控件、响应式布局与训练、使用指南和连接配置样式
```

可继续查看各子目录的索引：

- [feedback/](feedback/DIRECTORY.md)：全局操作结果弹窗、去重与面板反馈回归测试
- [guide/](guide/DIRECTORY.md)：控制面板集中使用指南与各功能操作步骤
- [resources/](resources/DIRECTORY.md)：角色与音色功能的页面组装和共享状态

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 611192860964192a9046bc218d6946e2b485b6dbb95292e2afff426d0f0fd5bb -->
