import { lstat, readFile, stat } from 'node:fs/promises';
import { isAbsolute, join } from 'node:path';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const execute = promisify(execFile);
export const asrSelectionPath = (root, override) => {
  if (override && !isAbsolute(override)) throw new Error('语音识别模型选择文件须使用绝对路径。');
  return override || join(root, 'config/local/asr-model-selection.json');
};
export const whisperIds = new Set(['faster-whisper-large-v3-turbo', 'faster-whisper-large-v3']);
export async function readAsrSelection(root, override) {
  const path = asrSelectionPath(root, override);
  let info;
  try { info = await lstat(path); } catch (error) { if (error.code === 'ENOENT') return null; throw error; }
  if (!info.isFile() || info.size > 16384) throw new Error('语音识别模型选择文件无效，请重新选择模型。');
  const value = JSON.parse(await readFile(path, 'utf8'));
  if (value.schema !== 1 || !whisperIds.has(value.model_id) || typeof value.id !== 'string'
      || typeof value.path !== 'string' || !isAbsolute(value.path)) throw new Error('语音识别模型选择文件无效，请重新选择模型。');
  return value;
}
export async function inspectWhisperModel(root) {
  const missing = [];
  for (const file of ['model.bin', 'config.json', 'tokenizer.json', 'preprocessor_config.json']) {
    try { const info = await stat(join(root, file)); if (!info.isFile() || info.size < (file === 'model.bin' ? 1024 : 1)) missing.push(file); }
    catch { missing.push(file); }
  }
  return { ready: missing.length === 0, missing };
}
export async function probeWhisperPython(python) {
  try {
    await execute(python, ['-B', '-s', '-c', 'import faster_whisper'], { timeout: 15000, maxBuffer: 4096,
      env: { ...process.env, PYTHONDONTWRITEBYTECODE: '1', HF_HUB_OFFLINE: '1' } });
    return true;
  } catch { return false; }
}
