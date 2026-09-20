import type { AgentPhase } from "@meowlive/contracts";
import { createAgentClient } from "../../services/server/agent";
import type { AgentClient } from "../../services/server/agent";
import { AgentSettingsForm } from "./AgentSettingsForm";
import { EventHistory } from "./EventHistory";
import { EventSimulator } from "./EventSimulator";
import { useAgentController } from "./useAgentController";

const defaultClient = createAgentClient();
const phaseLabels: Record<AgentPhase, string> = {
  paused: "Agent 已暂停",
  waiting: "Agent 等待事件",
  deciding: "Agent 正在决策",
  speaking: "Agent 正在发言",
};

export function AgentPanel({ client = defaultClient, pollIntervalMs = 1_000 }: { client?: AgentClient; pollIntervalMs?: number }) {
  const controller = useAgentController(client, pollIntervalMs);
  const { status, connectionError, actionError, pendingAction } = controller;
  const phase = status ? (status.paused ? "paused" : status.phase) : null;
  const canResume = Boolean(status?.paused && status.llm_configured && status.bridge_connected && !pendingAction && !connectionError);
  const canPause = Boolean(status && !status.paused && !pendingAction);

  return (
    <div className="agent-workspace" aria-labelledby="agent-heading">
      <section className="connection-card agent-status-card">
        <div>
          <h2 id="agent-heading">{phase ? phaseLabels[phase] : "正在读取 Agent 状态…"}</h2>
          <p className="server-address">{client.baseUrl}</p>
        </div>
        <div className="agent-status-actions">
          <div className="connection-indicators" aria-label="Agent 运行条件">
            <span className={`connection-pill ${status?.llm_configured && !connectionError ? "connected" : "disconnected"}`}>
              {status?.llm_configured ? "LLM 已配置" : "LLM 未配置"}
            </span>
            <span className={`connection-pill ${status?.bridge_connected && !connectionError ? "connected" : "disconnected"}`}>
              {status?.bridge_connected ? "桌面执行端已连接" : "桌面执行端未连接"}
            </span>
          </div>
          {status?.paused
            ? <button className="primary-button" type="button" disabled={!canResume} onClick={() => { void controller.resume(); }}>恢复 Agent</button>
            : <button className="stop-button" type="button" disabled={!canPause} onClick={() => { void controller.pause(); }}>暂停 Agent</button>}
        </div>
      </section>

      {(connectionError || actionError || status?.last_error) && <div className="error-banner" role="alert">
        {connectionError && <p>{connectionError}</p>}
        {actionError && <p>{actionError}</p>}
        {status?.last_error && <p>{status.last_error}</p>}
      </div>}
      {status && !status.llm_configured && <p className="availability-note">请先<a href="#llm">前往 LLM 接入</a>，重启主服务后恢复 Agent。</p>}
      {status && status.llm_configured && !status.bridge_connected && <p className="availability-note">请连接桌面执行端，再恢复 Agent。</p>}

      {status ? <>
        <div className="workspace-columns agent-config-grid">
          <AgentSettingsForm settings={status.settings} disabled={pendingAction !== null} onSave={controller.saveSettings} />
          <EventSimulator disabled={pendingAction !== null || Boolean(connectionError)} onSubmit={controller.submitEvents} />
        </div>
        <EventHistory events={status.events} />
      </> : <section className="panel"><div className="empty-state"><p>正在连接主服务</p><span>Agent 状态可用后会显示控制项。</span></div></section>}
    </div>
  );
}
