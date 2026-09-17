import { MAX_VOICE_FILE_BYTES, UnsupportedWavFormatError, validateVoiceWav, type ValidatedWav } from "./wav";

export const AUDIO_FILE_ACCEPT = ".mp3,.wav,.flac,.ogg,.oga,.opus,.m4a,.aac,.webm,audio/mpeg,audio/wav,audio/x-wav,audio/flac,audio/ogg,audio/mp4,audio/aac,audio/webm";
export const AUDIO_FORMAT_LABEL = "MP3、WAV、FLAC、OGG、M4A、AAC、WebM";
export const MAX_AUDIO_SOURCE_BYTES = 20 * 1024 * 1024;

/** Keep valid WAVs unchanged; normalize other browser-decodable audio before upload. */
export async function prepareAudioFile(file: File): Promise<{ file: File; info: ValidatedWav }> {
  if (file.size === 0) throw new Error("音频文件不能为空");
  if (file.size > MAX_AUDIO_SOURCE_BYTES) throw new Error("原始音频文件不能超过 20 MiB");
  const bytes = await file.arrayBuffer();
  const view = new DataView(bytes);
  const isWav = bytes.byteLength >= 12
    && view.getUint32(0) === 0x52494646 && view.getUint32(8) === 0x57415645;
  if (isWav && file.size <= MAX_VOICE_FILE_BYTES) {
    try {
      return { file, info: await validateVoiceWav(file) };
    } catch (error) {
      // Invalid containers, silence and duration errors must not be hidden by a decoder.
      if (!(error instanceof UnsupportedWavFormatError)) throw error;
    }
  }
  if (typeof OfflineAudioContext === "undefined") {
    throw new Error("当前浏览器不支持音频转换，请使用新版浏览器或选择 PCM16 WAV 文件");
  }
  let decoded: AudioBuffer;
  try {
    // Offline decoding needs neither an audio device nor playback permission.
    decoded = await new OfflineAudioContext(2, 1, 48_000).decodeAudioData(bytes);
  } catch {
    throw new Error("音频文件损坏或当前浏览器不支持此编码，请换用 MP3 或 WAV 文件");
  }
  const wav = encodeWav(decoded);
  const name = (file.name.replace(/\.[^.]+$/u, "") || "audio") + ".wav";
  const prepared = new File([wav], name, { type: "audio/wav" });
  return { file: prepared, info: await validateVoiceWav(prepared) };
}

function encodeWav(audio: AudioBuffer): ArrayBuffer {
  const { length, sampleRate, numberOfChannels: channels } = audio;
  if (![1, 2].includes(channels)) throw new Error("音频必须为单声道或双声道");
  if (length < 3 * sampleRate || length > 10 * sampleRate) throw new Error("参考音频需为 3–10 秒");
  const dataSize = length * channels * 2;
  if (dataSize + 44 > MAX_VOICE_FILE_BYTES) throw new Error("转换后的音频不能超过 2 MiB");
  const buffer = new ArrayBuffer(44 + dataSize);
  const view = new DataView(buffer);
  const write = (offset: number, value: string) => {
    for (let i = 0; i < value.length; i++) view.setUint8(offset + i, value.charCodeAt(i));
  };
  write(0, "RIFF");
  view.setUint32(4, buffer.byteLength - 8, true);
  write(8, "WAVEfmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, channels, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate * channels * 2, true);
  view.setUint16(32, channels * 2, true);
  view.setUint16(34, 16, true);
  write(36, "data");
  view.setUint32(40, dataSize, true);
  const planes = Array.from({ length: channels }, (_, channel) => audio.getChannelData(channel));
  for (let frame = 0; frame < length; frame++) {
    for (let channel = 0; channel < channels; channel++) {
      const value = planes[channel][frame];
      if (!Number.isFinite(value)) throw new Error("音频包含无效采样数据");
      const sample = Math.max(-1, Math.min(1, value));
      view.setInt16(44 + (frame * channels + channel) * 2, Math.round(sample * (sample < 0 ? 32768 : 32767)), true);
    }
  }
  return buffer;
}
