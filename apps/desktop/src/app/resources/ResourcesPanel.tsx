import { useContext, useEffect, useMemo, useRef } from "react";
import { createTrainingClient, type TrainingClient } from "../../services/server/training";
import { createServerClient } from "../../services/server";
import type { ServerClient } from "../../services/server";
import { createResourceClient } from "../../services/server/resources";
import type { ResourcesClient } from "../../services/server/resources";
import { CharacterPanel } from "../../features/characters";
import { VoicePanel } from "../../features/voices";
import { PersonaCardPanel } from "../../features/agent/PersonaCardPanel";
import { createAgentClient, type AgentClient } from "../../services/server/agent";
import { useResourcesController } from "./useResourcesController";
import { WorkspaceActiveContext } from "../Workspace";

const defaultResourceClient = createResourceClient();
const defaultSpeechClient = createServerClient();
const defaultAgentClient = createAgentClient();

export function ResourcesPanel({ resourceClient = defaultResourceClient, speechClient = defaultSpeechClient, trainingClient, agentClient = defaultAgentClient, mode = "characters", refreshToken = 0 }: {
  resourceClient?: ResourcesClient;
  speechClient?: ServerClient;
  trainingClient?: TrainingClient;
  agentClient?: AgentClient;
  mode?: "characters" | "voices";
  refreshToken?: number;
}) {
  const controller = useResourcesController(resourceClient, speechClient);
  const transcriber = useMemo(() => trainingClient ?? createTrainingClient({ baseUrl: resourceClient.baseUrl }), [trainingClient, resourceClient.baseUrl]);
  const activePage = useContext(WorkspaceActiveContext);
  const ownPage = mode === "characters" ? "resources" : "training";
  const wasActive = useRef(activePage === ownPage);
  const lastRefreshToken = useRef(refreshToken);
  useEffect(() => {
    if (activePage !== ownPage) { wasActive.current = false; return; }
    if (wasActive.current || controller.pendingAction) return;
    wasActive.current = true;
    void controller.refreshSnapshot();
  }, [activePage, ownPage, controller.pendingAction]);
  useEffect(() => {
    if (lastRefreshToken.current === refreshToken || controller.pendingAction) return;
    lastRefreshToken.current = refreshToken;
    void controller.refreshSnapshot();
  }, [refreshToken, controller.pendingAction]);

  return <div className="resources-workspace" aria-label={mode === "characters" ? "角色管理与人物卡" : "音色管理"}>
    {controller.loading && <p className="availability-note" role="status">正在读取{mode === "characters" ? "角色" : "音色"}资源…</p>}
    {(controller.loadError || controller.actionError) && <div className="error-banner" role="alert">
      {controller.loadError && <p>{controller.loadError}</p>}
      {controller.actionError && <p>{controller.actionError}</p>}
    </div>}
    {mode === "characters" ? <div className="workspace-columns resource-columns">
      {controller.snapshot && <CharacterPanel controller={controller} />}
      <PersonaCardPanel client={agentClient} />
    </div> : controller.snapshot && <VoicePanel key={resourceClient.baseUrl} controller={controller} transcriber={transcriber} />}
  </div>;
}
