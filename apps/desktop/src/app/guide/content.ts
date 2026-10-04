import type { WorkspacePage } from "../navigation";

type GuideTopic = { steps: string[]; note?: string };

export const featureGuides: Record<Exclude<WorkspacePage, "guide">, GuideTopic> = {
  overview: {
    steps: [
      "Windows 安装版：打开 App，内置主服务和桌面执行端自动启动。",
      "源码版：Windows 双击 launchers/start-windows.cmd；Linux / WSL 运行 ./launchers/start.sh，保持终端打开。",
      "在运行总览查看状态；遇到问题到「运行日志」查看原因。",
      "退出安装版直接关闭 App；源码版在启动终端按 Ctrl+C。",
    ],
  },
  setup: {
    steps: [
      "Windows 安装版可在“环境与模型”检测或安装 WSL2 语音后端、下载模型；Docker 和数据库按需要另行准备。",
      "源码版先准备 GPT-SoVITS 引擎，在 config/local/launcher.json 配置 Python 和引擎目录。",
      "在本地模型中选择 GPT-SoVITS v2；缺少权重时下载后重新扫描。标记「需适配」的模型暂不能直接使用。",
      "需要观众存储时，展开「数据库与可选功能」，按步骤准备 PostgreSQL + pgvector 并启用。",
    ],
    note: "原生 Windows App 基础功能不需要 WSL；声音环境向导可补齐 WSL2 语音后端，保留原有主服务和用户数据。",
  },
  speech: {
    steps: [
      "确认 TTS、语音模型和桌面执行端已就绪，并已选用音色。",
      "输入文字，点击「加入播报队列」，查看合成与播放状态。",
      "需要中断时点击「停止全部播报」。",
    ],
  },
  resources: {
    steps: [
      "在 VTube Studio 加载模型，开启插件 API 并允许连接。",
      "新建角色，选择模型和音色，保存后选用。嘴型输入使用 MeowMouthOpen，范围 0–1。",
      "填写人物卡的核心身份，其他字段按需补充；可新建、切换、重命名或删除人物卡。",
    ],
    note: "角色形象和人物卡分别选择。保存人物卡后，到「Agent 互动」恢复 Agent。",
  },
  agent: {
    steps: [
      "先完成「LLM 接入」和人物卡设置。",
      "填写直播话题，调整弹幕、欢迎和冷却选项，保存后点击「恢复 Agent」。",
      "发送模拟事件验证回复；正式直播时到「直播连接」接入。",
    ],
  },
  "agent-observability": {
    steps: [
      "查看调度状态，确认 Agent 是否暂停、冷却或等待资源。",
      "选择一条 Trace，查看模型调用、工具执行和语音播放过程。",
      "失败时从最后一个异常步骤定位问题，再到对应页面处理。",
    ],
  },
  viewers: {
    steps: [
      "先准备 PostgreSQL + pgvector 并启用观众存储；Windows 安装版默认关闭。",
      "查看观众与事件，打开「管理详情」管理积分、礼物和记忆。",
      "修改、合并或删除记录前核对对象；要求填写原因时如实填写。",
    ],
    note: "源码启动器可自动准备项目数据库，但需要 Docker 已运行。记忆模型与 Neo4j 按需另行配置。",
  },
  llm: {
    steps: [
      "填写 API 地址和密钥，获取并选择模型。",
      "测试连接，成功后保存配置并重启 App 或源码启动器。测试可能产生调用费用。",
      "需要时调整推理强度、工具和搜索设置；可保存多套连接配置。",
    ],
  },
  live: {
    steps: [
      "准备 B 站直播开放平台的应用 ID、AccessKey ID、AccessKey Secret 和主播身份码。",
      "填写并保存，点击「连接直播间」，确认连接成功。",
      "到「Agent 互动」恢复 Agent；结束时断开直播间。",
    ],
    note: "仅填写房间号不能接入。",
  },
  obs: {
    steps: [
      "在 OBS 中启用 WebSocket 服务器，填写连接地址和密码并保存。",
      "刷新状态后切换场景或控制录制。",
      "在 OBS 采集 VTube Studio 画面和播放声音；结束时单独停止录制。",
    ],
  },
  training: {
    steps: [
      "先上传一段 3–10 秒清晰参考录音，核对原文并选用音色。不训练也能试播。",
      "需要训练时，在环境与模型安装后端，下载、选用模型并连接训练后端；上传同一人的 2–32 段录音，每段 3–10 秒。",
      "选择本地语音识别模型，或手工填写并校对文本。关闭语音模型释放资源后开始训练。",
      "完成后启用语音模型，试听新版本，再点击「确认所选音色」应用。",
    ],
    note: "Windows 主服务通过 WSL2 执行训练，需要可用 CUDA GPU。环境与模型页也支持 GitHub 版本检查、差量下载和安装更新。",
  },
  logs: {
    steps: [
      "按级别、来源、分类或关键词筛选，最多查看最新 1000 条。",
      "可暂停自动刷新或手动刷新；Agent 的详细执行过程在「Agent 观察」查看。",
    ],
  },
};
