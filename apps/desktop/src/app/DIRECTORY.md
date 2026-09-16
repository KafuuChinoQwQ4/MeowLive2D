# app 目录索引

React 根页面组装与全局样式

本文件由 `npm run tree:update` 生成，覆盖当前目录的全部受维护子目录。每项右侧为大致用途。

```text
app/  # React 根页面组装与全局样式
├── resources/  # 角色与音色功能的页面组装和共享状态
│   ├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
│   ├── ResourcesPanel.lifecycle.test.tsx  # 资源读取与操作的卸载取消测试
│   ├── ResourcesPanel.regressions.test.tsx  # 当前角色重新加载与安装成功后刷新失败回归
│   ├── ResourcesPanel.tsx  # 组装角色与音色面板并显示资源操作状态
│   ├── index.ts  # 资源管理页面公共入口
│   └── useResourcesController.ts  # 资源快照与桌面操作的共享控制器
├── App.desktop.test.tsx  # 原生桌面地址初始化、导航功能请求地址与失败回归
├── App.launcher.test.tsx  # 服务启停后的业务门控、环境页面常驻访问与模型检索草稿保留测试
├── App.navigation.test.tsx  # 切页保留在途播报、历史深链接与未知页面回退的集成测试
├── App.test.tsx  # 导航功能显隐、草稿保留与当前页标识的集成测试
├── App.tsx  # 组装客户端与原生配置，选择受管或手动模式的导航控制台
├── DIRECTORY.md  # 本目录递归目录树、文件用途与同步指纹（自动生成）
├── ManagedWorkspace.tsx  # 共享启动管理状态、模型操作会话及业务就绪门控的工作区
├── Workspace.tsx  # 固定侧栏、始终可访问的环境页面、快捷入口与稳定挂载的导航布局
├── WorkspaceIcon.tsx  # 控制台导航、品牌猫形与快捷操作的代码内 SVG 图标
├── navigation.ts  # 控制台功能导航、分组、页面说明与 URL fragment 映射
├── styles.css  # 控制台公共样式、响应式布局与服务滑动开关样式
├── useWorkspaceNavigation.ts  # 页面选择、访问记录与浏览器前进后退同步
└── workspace.css  # 粉色亚克力导航工作区、圆角下拉与文件选择控件、半透明卡片阴影和环境模型页面的响应式样式
```

可继续查看各子目录的索引：

- [resources/](resources/DIRECTORY.md)：角色与音色功能的页面组装和共享状态

用途说明源：`scripts/directory-descriptions.json`。新增、删除、移动文件或调整职责时先同步说明源，再运行生成命令。

已有文件内容变化也会更新下方指纹；用途未变时保留原说明。检查命令 `npm run tree:check` 只检查，不修改文件。

<!-- directory-tree-sha256: 1cbdecaaaaa2d8dbdd6f2237eb5464b69d6ced7415eb2a96925bc42ea3dfb3b0 -->
