import { useState } from "react";
import type { LlmRuntimeClient } from "../../services/server/llm-runtime";
import { RuntimeSettingsPanel, type LlmConnectionIdentity } from "./RuntimeSettingsPanel";
import { UsagePanel } from "./UsagePanel";
import { useRuntimeVisibility } from "./useRuntimeVisibility";
import "./llm-runtime.css";

export function LlmRuntimePanel({ client, connection }: { client: LlmRuntimeClient; connection?: LlmConnectionIdentity }) {
  const { ref, visible } = useRuntimeVisibility();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [usageOpen, setUsageOpen] = useState(false);
  const [settingsVisited, setSettingsVisited] = useState(false);
  const [usageVisited, setUsageVisited] = useState(false);
  const [revision, setRevision] = useState(0);
  return <div ref={ref} className="llm-runtime">
    <details className="panel runtime-details" onToggle={event => { setSettingsOpen(event.currentTarget.open); if (event.currentTarget.open) setSettingsVisited(true); }}>
      <summary>运行能力与搜索</summary>
      {settingsVisited && <RuntimeSettingsPanel client={client} connection={connection} active={visible && settingsOpen} onSaved={() => setRevision(value => value + 1)} />}
    </details>
    <details className="panel runtime-details" onToggle={event => { setUsageOpen(event.currentTarget.open); if (event.currentTarget.open) setUsageVisited(true); }}>
      <summary>用量与费用</summary>
      {usageVisited && <UsagePanel client={client} active={visible && usageOpen} revision={revision} />}
    </details>
  </div>;
}
