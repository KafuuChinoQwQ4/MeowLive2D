import type { ResourceSnapshot, SpeechSnapshot, VoiceCreateRequest } from "@meowlive/contracts";

/** 音色面板依赖的状态与操作，由 app 组装。 */
export interface VoiceController {
  snapshot: ResourceSnapshot | null;
  pendingAction: string | null;
  voicePreview: SpeechSnapshot | null;
  voiceDeleteRetries: string[];
  selectVoice(id: string): Promise<ResourceSnapshot | null>;
  deleteVoice(id: string): Promise<ResourceSnapshot | null>;
  uploadVoice(metadata: VoiceCreateRequest, audio: File): Promise<ResourceSnapshot | null>;
  previewVoice(id: string, text: string): Promise<SpeechSnapshot | null>;
}
