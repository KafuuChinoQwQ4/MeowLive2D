import { useMemo } from "react";
import { createTrainingClient, type TrainingClient } from "../../services/server/training";
import { createServerClient } from "../../services/server";
import type { ServerClient } from "../../services/server";
import { createResourceClient } from "../../services/server/resources";
import type { ResourcesClient } from "../../services/server/resources";
import { CharacterPanel } from "../../features/characters";
import { VoicePanel } from "../../features/voices";
import { useResourcesController } from "./useResourcesController";

const defaultResourceClient = createResourceClient();
const defaultSpeechClient = createServerClient();

export function ResourcesPanel({ resourceClient = defaultResourceClient, speechClient = defaultSpeechClient, trainingClient }: {
  resourceClient?: ResourcesClient;
  speechClient?: ServerClient;
  trainingClient?: TrainingClient;
}) {
  const controller = useResourcesController(resourceClient, speechClient);
  const transcriber = useMemo(() => trainingClient ?? createTrainingClient({ baseUrl: resourceClient.baseUrl }), [trainingClient, resourceClient.baseUrl]);

  return <div className="resources-workspace" aria-label="资源管理">
    {controller.loading && <p className="availability-note" role="status">正在读取角色与音色资源…</p>}
    {(controller.loadError || controller.actionError) && <div className="error-banner" role="alert">
      {controller.loadError && <p>{controller.loadError}</p>}
      {controller.actionError && <p>{controller.actionError}</p>}
    </div>}
    {controller.snapshot && <div className="workspace-columns resource-columns">
      <VoicePanel key={resourceClient.baseUrl} controller={controller} transcriber={transcriber} />
      <CharacterPanel controller={controller} />
    </div>}
  </div>;
}
