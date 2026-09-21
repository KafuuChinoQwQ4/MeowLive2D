import { useEffect, useState } from "react";
import type { AgentActivitySnapshot } from "@meowlive/contracts";
import type { LlmRuntimeClient } from "../../services/server/llm-runtime";
import { useRuntimeVisibility } from "./useRuntimeVisibility";
import "./llm-runtime.css";

const phaseLabels: Record<string, string> = { idle: "等待互动", thinking: "正在思考", receiving: "正在接收回复", completed: "本轮已完成", failed: "本轮未完成", cancelled: "本轮已取消" };
const toolLabels: Record<string, string> = { web_search: "网页搜索", get_current_time: "当前时间", get_environment: "直播与播放状态", get_obs_status: "OBS 状态" };
const statusLabels: Record<string, string> = { running: "进行中", completed: "已完成", failed: "未成功" };

export function AgentActivityPanel({ client, pollIntervalMs = 1000 }: { client: LlmRuntimeClient; pollIntervalMs?: number }) {
  const { ref, visible } = useRuntimeVisibility();
  const [activity, setActivity] = useState<AgentActivitySnapshot | null>(null);
  const [error, setError] = useState(false);
  useEffect(() => { setActivity(null); setError(false); }, [client]);
  useEffect(() => {
    if (!visible) return;
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout> | undefined;
    async function refresh() {
      try {
        const next = await client.getActivity(controller.signal);
        if (controller.signal.aborted) return;
        setActivity(next); setError(false);
      } catch { if (!controller.signal.aborted) setError(true); }
      if (!controller.signal.aborted) timer = setTimeout(() => { void refresh(); }, pollIntervalMs);
    }
    void refresh();
    return () => { controller.abort(); clearTimeout(timer); };
  }, [client, pollIntervalMs, visible]);
  const currentTools = activity?.tools.filter(tool => tool.status === "running") ?? [];
  const phase = activity?.phase === "tool" ? (currentTools.some(tool => tool.name === "web_search") ? "正在查资料" : "正在读取环境") : phaseLabels[activity?.phase ?? ""];
  const elapsed = activity?.started_at_ms === null || !activity ? null : Math.max(0, activity.updated_at_ms - activity.started_at_ms) / 1000;
  return <div ref={ref} className="panel runtime-activity" aria-label="Agent 当前活动">
    <div className="runtime-section-heading"><h3>当前活动</h3><span className="runtime-badge" role="status">{error ? "活动暂不可用" : phase || "正在读取活动…"}</span></div>
    {error && <p className="field-hint">暂时无法更新活动，连接恢复后会自动刷新。</p>}
    {activity && <>
      <p className="muted">{elapsed !== null && `耗时 ${elapsed.toFixed(1)} 秒 · `}工具轮次 {activity.tool_round} · 已接收 {activity.output_characters.toLocaleString("zh-CN")} 字符</p>
      <p className="field-hint">完整回复校验完成后才会播报。</p>
      {activity.tools.length > 0 && <ul className="runtime-tool-list">{activity.tools.map((tool, index) => <li key={`${index}-${tool.name}`}>
        <div className="runtime-section-heading"><strong>{toolLabels[tool.name] ?? "读取资料"}</strong><span>{statusLabels[tool.status]} · {(tool.elapsed_ms / 1000).toFixed(1)} 秒</span></div>
        {tool.sources.length > 0 && <div className="runtime-sources"><span>来源</span>{tool.sources.map((source, sourceIndex) => <a key={`${sourceIndex}-${source}`} href={source} title={source} target="_blank" rel="noopener noreferrer" referrerPolicy="no-referrer">{new URL(source).hostname}</a>)}</div>}
      </li>)}</ul>}
    </>}
  </div>;
}
