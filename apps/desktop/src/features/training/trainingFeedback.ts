import type { TrainingJob } from "@meowlive/contracts";
import { ServerRequestError } from "../../services/server/responses";

export type TrainingFeedback = { kind: "success" | "error" | "info"; title: string; message: string; tips: string[] };
export type TrainingNotice = TrainingFeedback & { id: number; restoreFocus?: HTMLElement };
export type ActionFeedback = { operation: string; success: string; successTitle?: string };

export function failureFeedback(operation: string, error: unknown): TrainingFeedback {
  const message = error instanceof Error ? error.message : "操作未完成，服务没有提供具体原因";
  const code = error instanceof ServerRequestError ? error.code : "";
  let tips: string[];
  if (code === "gpu_unavailable" || /GPU.*采样|nvidia-smi|显存.*无效|GPU 序号/.test(message)) {
    tips = ["在运行主服务的 Linux / WSL 终端执行 nvidia-smi，确认能列出显卡。", "核对高级设置中的 GPU 编号；只有一张显卡时通常选择 0。", "如果显卡无法识别，请检查 NVIDIA 驱动及 WSL 的 GPU 支持，再重新尝试。"];
  } else if (code === "gpu_occupied" || /显存|out of memory|CUDA.*memory/i.test(message)) {
    tips = ["训练前关闭语音模型和本地 LLM，释放显存。", "降低每批片段数，选择“省内存”预设并开启“节省显存”；离线测量可尝试更小的本地模型。"];
  } else if (code === "inference_running" || /释放.*GPU|推理进程/.test(message)) {
    tips = ["训练前点击“关闭语音模型”，并停止本地 LLM。旧版或外部 TTS 需要在原终端停止。", "等待正在进行的推理结束后重试。"];
  } else if (code === "session_busy" || code === "gpu_busy" || /忙碌|正在.*(?:播报|合成|训练|推理)/.test(message)) {
    tips = ["等待当前任务结束；训练前结束直播并暂停 Agent 自动回应。", "在训练记录中确认任务状态，避免重复提交。"];
  } else if (code === "request_timeout" || /超时/.test(message)) {
    tips = ["先查看训练记录或模型状态，确认后台任务是否仍在执行，不要连续重复提交。", "等待片刻后再试；持续超时可检查“启动与运行”中的服务日志。"];
  } else if (code === "connection_failed" || /无法连接|未就绪|未启用|未启动|模型加载失败|TTS 请求失败/.test(message)) {
    tips = ["在“启动与运行”确认主服务与 TTS 状态；试听和离线测量还需要单独启用语音模型。", "检查服务日志，处理报告的问题后重新尝试。"];
  } else if (/文本|识别|转写|transcri/i.test(message)) {
    tips = ["检查本地语音识别模型是否配置完整；也可选择“输入并校对文本”后手工填写。", "确认录音内容、语言和文字一致，补全空白文本后重试。"];
  } else if (/音频|片段|静音|解码|MiB/.test(message)) {
    tips = ["选择 2–32 段清晰的非静音录音，每段 3–10 秒；检查原文件及转换后总大小。", "无法解码时可重新导出为 WAV，再导入训练片段。"];
  } else if (/文件|目录|磁盘|存储|权重/.test(message)) {
    tips = ["检查磁盘空间、目录写入权限，以及模型文件是否完整。", "删除操作失败时，先查看列表当前状态，再使用“重试删除”，避免误删其他版本。"];
  } else if (code === "measurement_unverified" || /实测未通过|采样完整/.test(message)) {
    tips = ["根据测量结果检查显存余量和耗时，可关闭其他高占用程序或选择更小的本地模型。", "确认 LLM、TTS 与显卡采样均可用后，再手动测量。"];
  } else {
    tips = ["先按上面的原因检查当前配置和素材，再重新尝试。", "仍失败时，到“启动与运行”查看服务日志；训练中途失败还可查看该任务的 engine.log 与 stages.log。"];
  }
  return { kind: "error", title: `${operation}失败`, message, tips };
}

export function terminalFeedback(job: TrainingJob): TrainingFeedback | null {
  if (job.status === "completed") return { kind: "success", title: "训练完成", message: `“${job.name}”已完成训练。${job.message}`, tips: ["到“已训练音色”启用语音模型并试听，确认效果后保存和选用。"] };
  if (job.status === "failed") return { ...failureFeedback("训练", new Error(job.message)), message: `“${job.name}”：${job.message}` };
  if (job.status === "interrupted") return { ...failureFeedback("训练", new Error(job.message)), title: "训练已中断", message: `“${job.name}”：${job.message}` };
  if (job.status === "cancelled") return { kind: "info", title: "训练已取消", message: `“${job.name}”已停止。${job.message}`, tips: ["可在训练记录中检查结果，调整参数后新建任务。"] };
  return null;
}
