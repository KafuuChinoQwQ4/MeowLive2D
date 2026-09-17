import { afterEach, describe, expect, it, vi } from "vitest";
import { pcm16Wav } from "../../test/resource-fixtures";
import { MAX_AUDIO_SOURCE_BYTES, prepareAudioFile } from "./index";
import { validateVoiceWav } from "./wav";

function decoder(options: { seconds?: number; channels?: number; samples?: number[] } = {}) {
  const sampleRate = 48_000;
  const length = Math.round((options.seconds ?? 4) * sampleRate);
  const channels = options.channels ?? 1;
  const planes = Array.from({ length: channels }, (_, channel) => {
    const data = new Float32Array(length).fill(channel === 0 ? 0.25 : -0.5);
    if (options.samples) data.set(options.samples);
    return data;
  });
  const decode = vi.fn().mockResolvedValue({
    sampleRate, length, numberOfChannels: channels,
    getChannelData: (channel: number) => planes[channel],
  });
  vi.stubGlobal("OfflineAudioContext", class {
    decodeAudioData = decode;
  });
  return decode;
}

afterEach(() => vi.unstubAllGlobals());

describe("多格式音频导入", () => {
  it("保留已经合规的 WAV 文件与采样率，不调用解码器", async () => {
    const decode = decoder();
    const file = pcm16Wav();
    const result = await prepareAudioFile(file);
    expect(result.file).toBe(file);
    expect(result.info).toEqual({ durationMs: 4000, sampleRate: 8000, channels: 1 });
    expect(decode).not.toHaveBeenCalled();
  });

  it("将解码音频转换成有效 PCM16 WAV，保留声道顺序并限制样本振幅", async () => {
    decoder({ channels: 2, samples: [-2, -1, 0, 0.5, 1, 2] });
    const result = await prepareAudioFile(new File(["compressed audio"], "声音.MP3", { type: "audio/mpeg" }));
    expect(result.file.name).toBe("声音.wav");
    expect(result.file.type).toBe("audio/wav");
    expect(result.info).toEqual({ durationMs: 4000, sampleRate: 48000, channels: 2 });
    await expect(validateVoiceWav(result.file)).resolves.toEqual(result.info);
    const view = new DataView(await result.file.arrayBuffer());
    expect(Array.from({ length: 6 }, (_, i) => view.getInt16(44 + i * 4, true)))
      .toEqual([-32768, -32768, 0, 16384, 32767, 32767]);
    expect(view.getInt16(44 + 6 * 4 + 2, true)).toBe(-16384);
  });

  it("自动转换非 PCM16 WAV，而非要求用户手动修改位深", async () => {
    const source = await pcm16Wav().arrayBuffer();
    new DataView(source).setUint16(34, 24, true);
    const decode = decoder();
    const result = await prepareAudioFile(new File([source], "24bit.wav"));
    expect(decode).toHaveBeenCalledOnce();
    await expect(validateVoiceWav(result.file)).resolves.toMatchObject({ sampleRate: 48000 });
  });

  it.each([2, 11])("拒绝解码后 %s 秒的片段", async seconds => {
    decoder({ seconds });
    await expect(prepareAudioFile(new File(["audio"], "clip.mp3"))).rejects.toThrow(/3–10 秒/);
  });

  it("拒绝静音、损坏 WAV 和多声道素材", async () => {
    const decode = decoder({ channels: 6 });
    await expect(prepareAudioFile(pcm16Wav({ amplitude: 0 }))).rejects.toThrow(/静音/);
    const truncated = await pcm16Wav().arrayBuffer();
    await expect(prepareAudioFile(new File([truncated.slice(0, -2)], "broken.wav"))).rejects.toThrow(/有效|完整/);
    expect(decode).not.toHaveBeenCalled();
    await expect(prepareAudioFile(new File(["audio"], "surround.flac"))).rejects.toThrow(/单声道或双声道/);
  });

  it("拒绝量化后静音或包含无效样本的解码结果", async () => {
    const decode = decoder();
    for (const amplitude of [0, 1e-9, Number.NaN]) {
      decode.mockResolvedValueOnce({ sampleRate: 48000, length: 144000, numberOfChannels: 1,
        getChannelData: () => new Float32Array(144000).fill(amplitude) });
      await expect(prepareAudioFile(new File(["audio"], "clip.ogg"))).rejects.toThrow(/静音|无效/);
    }
  });

  it("在解码前拒绝空文件和过大文件", async () => {
    const decode = decoder();
    await expect(prepareAudioFile(new File([], "empty.mp3"))).rejects.toThrow(/空/);
    await expect(prepareAudioFile(new File([new Uint8Array(MAX_AUDIO_SOURCE_BYTES + 1)], "large.flac")))
      .rejects.toThrow(/20 MiB/);
    expect(decode).not.toHaveBeenCalled();
  });

  it("给出浏览器不支持解码或文件损坏的可操作错误", async () => {
    const decode = decoder();
    decode.mockRejectedValueOnce(new DOMException("Unable to decode", "EncodingError"));
    await expect(prepareAudioFile(new File(["bad audio"], "broken.m4a"))).rejects.toThrow(/损坏|不支持/);
    vi.stubGlobal("OfflineAudioContext", undefined);
    await expect(prepareAudioFile(new File(["audio"], "clip.mp3"))).rejects.toThrow(/浏览器.*PCM16 WAV/);
  });
});
