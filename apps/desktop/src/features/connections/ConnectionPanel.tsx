import type { LiveConnectionPhase } from "@meowlive/contracts";
import { useState } from "react";
import { createLiveClient } from "../../services/server/live";
import type { LiveClient } from "../../services/server/live";
import { useConnectionController } from "./useConnectionController";
import { LiveSettingsForm } from "./LiveSettingsForm";

const defaultClient = createLiveClient();
const phaseLabels: Record<LiveConnectionPhase, string> = {
  disabled: "直播接入未启用",
  disconnected: "直播间未连接",
  connecting: "正在连接直播间…",
  connected: "直播间已连接",
  reconnecting: "正在重新连接…",
  disconnecting: "正在断开直播间…",
  failed: "直播连接失败",
};

function platformLabel(platform: string): string {
  return platform.toLowerCase() === "bilibili" ? "哔哩哔哩" : platform;
}

export function ConnectionPanel({ client = defaultClient, pollIntervalMs = 1_000 }: { client?: LiveClient; pollIntervalMs?: number }) {
  const controller = useConnectionController(client, pollIntervalMs);
  const [settingsSaving, setSettingsSaving] = useState(false);
  const { status, connectionError, actionError, pendingAction } = controller;
  const canConnect = Boolean(status?.configured
    && (status.phase === "disconnected" || status.phase === "failed")
    && !pendingAction && !settingsSaving);
  const canDisconnect = Boolean(status
    && ["connecting", "connected", "reconnecting"].includes(status.phase)
    && !pendingAction && !settingsSaving);
  const showDisconnect = status?.phase === "disconnecting" || canDisconnect || pendingAction === "disconnect";

  return (
    <section className="live-workspace" aria-labelledby="live-connection-heading">
      <div className="connection-card live-status-card">
        <div>
          <h2 id="live-connection-heading">{status ? phaseLabels[status.phase] : "正在读取直播连接状态…"}</h2>
          <p className="server-address">{client.baseUrl}</p>
        </div>
        <div className="live-status-actions">
          {status && <div className="connection-indicators" aria-label="直播连接信息">
            <span className={`connection-pill ${status.phase === "connected" && !connectionError ? "connected" : "disconnected"}`}>
              {platformLabel(status.platform)}
            </span>
            <span className="connection-pill disconnected">房间 {status.room_id ?? "待连接"}</span>
          </div>}
          {showDisconnect
            ? <button type="button" className="stop-button" disabled={!canDisconnect} onClick={() => { void controller.disconnect(); }}>
                {status?.phase === "disconnecting" || pendingAction === "disconnect" ? "正在断开…" : "断开直播间"}
              </button>
            : <button type="button" className="primary-button" disabled={!canConnect} onClick={() => { void controller.connect(); }}>
                {pendingAction === "connect" ? "正在连接…" : "连接直播间"}
              </button>}
        </div>
      </div>

      {(connectionError || actionError || status?.last_error) && <div className="error-banner" role="alert">
        {connectionError && <p>{connectionError}</p>}
        {actionError && <p>{actionError}</p>}
        {status?.last_error && <p>{status.last_error}</p>}
      </div>}
      {status && (!status.configured || status.phase === "disabled") && <p className="availability-note">
        请在下方填写直播配置并启用接入，保存后即可连接直播间。
      </p>}

      <LiveSettingsForm client={client} phase={status?.phase} actionPending={Boolean(pendingAction)} onSaved={controller.refresh} onSavingChange={setSettingsSaving} />

      {status && <dl className="live-metrics" aria-label="直播事件统计">
        <div><dt>平台</dt><dd>{platformLabel(status.platform)}</dd></div>
        <div><dt>房间</dt><dd>{status.room_id ?? "待连接"}</dd></div>
        <div><dt>已接收</dt><dd>{status.accepted_events}</dd></div>
        <div><dt>已去重</dt><dd>{status.duplicate_events}</dd></div>
        <div><dt>已丢弃</dt><dd>{status.rejected_events}</dd></div>
        <div><dt>重连次数</dt><dd>{status.reconnect_attempts}</dd></div>
      </dl>}
    </section>
  );
}
