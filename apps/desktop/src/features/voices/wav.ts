export interface ValidatedWav {
  durationMs: number;
  sampleRate: number;
  channels: number;
}

export const MAX_VOICE_FILE_BYTES = 2 * 1024 * 1024;

function chunkName(view: DataView, offset: number): string {
  return String.fromCharCode(
    view.getUint8(offset),
    view.getUint8(offset + 1),
    view.getUint8(offset + 2),
    view.getUint8(offset + 3),
  );
}

export async function validateVoiceWav(file: File): Promise<ValidatedWav> {
  if (file.size > MAX_VOICE_FILE_BYTES) throw new Error("WAV 文件不能超过 2 MiB");
  if (file.size < 44) throw new Error("请选择有效的 WAV 文件");

  const view = new DataView(await file.arrayBuffer());
  if (
    chunkName(view, 0) !== "RIFF" ||
    chunkName(view, 8) !== "WAVE" ||
    view.getUint32(4, true) + 8 !== view.byteLength
  ) {
    throw new Error("请选择有效的 WAV 文件");
  }

  let format: { sampleRate: number; channels: number; blockAlign: number } | null = null;
  let dataOffset = -1;
  let dataBytes = 0;
  for (let offset = 12; offset + 8 <= view.byteLength; ) {
    const size = view.getUint32(offset + 4, true);
    const start = offset + 8;
    const end = start + size;
    if (end > view.byteLength) throw new Error("WAV 文件结构不完整");
    const name = chunkName(view, offset);

    if (name === "fmt ") {
      if (format || size < 16) throw new Error("WAV 音频格式无效");
      const audioFormat = view.getUint16(start, true);
      const channels = view.getUint16(start + 2, true);
      const sampleRate = view.getUint32(start + 4, true);
      const byteRate = view.getUint32(start + 8, true);
      const blockAlign = view.getUint16(start + 12, true);
      const bitsPerSample = view.getUint16(start + 14, true);
      if (
        audioFormat !== 1 ||
        bitsPerSample !== 16 ||
        ![1, 2].includes(channels) ||
        sampleRate < 8_000 ||
        sampleRate > 48_000 ||
        blockAlign !== channels * 2 ||
        byteRate !== sampleRate * blockAlign
      ) {
        throw new Error("仅支持 8–48 kHz、单声道或双声道的 PCM16 WAV");
      }
      format = { sampleRate, channels, blockAlign };
    } else if (name === "data") {
      if (dataOffset >= 0) throw new Error("WAV 音频数据无效");
      dataOffset = start;
      dataBytes = size;
    }
    offset = end + (size % 2);
  }

  if (!format || dataOffset < 0 || dataBytes === 0 || dataBytes % format.blockAlign !== 0) {
    throw new Error("WAV 音频数据无效");
  }
  const durationMs = (dataBytes / (format.sampleRate * format.blockAlign)) * 1_000;
  if (durationMs < 3_000 || durationMs > 10_000) {
    throw new Error("参考音频需为 3–10 秒");
  }

  let audible = false;
  for (let offset = dataOffset; offset < dataOffset + dataBytes; offset += 2) {
    if (view.getInt16(offset, true) !== 0) {
      audible = true;
      break;
    }
  }
  if (!audible) throw new Error("参考音频不能是静音");

  return {
    durationMs: Math.round(durationMs),
    sampleRate: format.sampleRate,
    channels: format.channels,
  };
}
