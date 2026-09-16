import type {
  CharacterProfile,
  ResourceSnapshot,
  VoiceProfile,
  VtsHotkey,
  VtsModel,
} from "@meowlive/contracts";

export function voice(overrides: Partial<VoiceProfile> = {}): VoiceProfile {
  return {
    id: "voice-1",
    name: "温柔旁白",
    language: "zh",
    reference_text: "你好，欢迎来到直播间。",
    duration_ms: 4_000,
    sample_rate: 24_000,
    channels: 1,
    available: true,
    error: null,
    ...overrides,
  };
}

export function character(overrides: Partial<CharacterProfile> = {}): CharacterProfile {
  return {
    id: "character-1",
    name: "小猫主播",
    model_id: "model-1",
    voice_id: "voice-1",
    mouth_parameter: "ParamMouthOpenY",
    mappings: [{ intent: "挥手", hotkey_id: "hotkey-wave", fallback_hotkey_id: null, validated: false }],
    ...overrides,
  };
}

export function resourceSnapshot(overrides: Partial<ResourceSnapshot> = {}): ResourceSnapshot {
  return {
    voices: [voice()],
    characters: [character()],
    active_voice_id: "voice-1",
    active_character_id: "character-1",
    default_voice_available: true,
    ...overrides,
  };
}

export function model(overrides: Partial<VtsModel> = {}): VtsModel {
  return { id: "model-1", name: "Cat Model", ...overrides };
}

export function hotkey(overrides: Partial<VtsHotkey> = {}): VtsHotkey {
  return { id: "hotkey-wave", name: "Wave", ...overrides };
}

export function pcm16Wav(options: { seconds?: number; sampleRate?: number; channels?: number; amplitude?: number } = {}): File {
  const seconds = options.seconds ?? 4;
  const sampleRate = options.sampleRate ?? 8_000;
  const channels = options.channels ?? 1;
  const amplitude = options.amplitude ?? 1_000;
  const samples = seconds * sampleRate * channels;
  const buffer = new ArrayBuffer(44 + samples * 2);
  const view = new DataView(buffer);
  const write = (offset: number, value: string) => [...value].forEach((char, index) => view.setUint8(offset + index, char.charCodeAt(0)));
  write(0, "RIFF");
  view.setUint32(4, buffer.byteLength - 8, true);
  write(8, "WAVE");
  write(12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, channels, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate * channels * 2, true);
  view.setUint16(32, channels * 2, true);
  view.setUint16(34, 16, true);
  write(36, "data");
  view.setUint32(40, samples * 2, true);
  for (let index = 0; index < samples; index += 1) view.setInt16(44 + index * 2, amplitude, true);
  return new File([buffer], "reference.wav", { type: "audio/wav" });
}
