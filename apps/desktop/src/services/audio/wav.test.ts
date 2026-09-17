import { describe, expect, it } from "vitest";
import { pcm16Wav } from "../../test/resource-fixtures";
import { validateVoiceWav } from "./wav";

describe("参考与训练音频 WAV 预检", () => {
  it("接受 3–10 秒、8–48kHz、单声道或双声道的非静音 PCM16 WAV", async () => {
    await expect(validateVoiceWav(pcm16Wav({ seconds: 3, sampleRate: 8_000, channels: 1 }))).resolves.toEqual({
      durationMs: 3_000, sampleRate: 8_000, channels: 1,
    });
    await expect(validateVoiceWav(pcm16Wav({ seconds: 10, sampleRate: 48_000, channels: 2 }))).resolves.toEqual({
      durationMs: 10_000, sampleRate: 48_000, channels: 2,
    });
  });

  it.each([
    ["过短", pcm16Wav({ seconds: 2 })],
    ["过长", pcm16Wav({ seconds: 11 })],
    ["静音", pcm16Wav({ amplitude: 0 })],
    ["采样率越界", pcm16Wav({ sampleRate: 4_000 })],
  ])("拒绝%s音频", async (_label, file) => {
    await expect(validateVoiceWav(file)).rejects.toThrow();
  });

  it("拒绝伪装成 WAV 的内容和超过 2 MiB 的文件", async () => {
    await expect(validateVoiceWav(new File(["not wav"], "fake.wav", { type: "audio/wav" }))).rejects.toThrow();
    await expect(validateVoiceWav(new File([new Uint8Array(2 * 1024 * 1024 + 1)], "large.wav"))).rejects.toThrow(/2 MiB/);
  });
});
