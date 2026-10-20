import { readFile, stat } from 'node:fs/promises';
import { release } from 'node:os';
import { join } from 'node:path';

export function detectEnvironment(kernel = release(), env = process.env) {
  const isWsl = /microsoft|wsl/i.test(kernel) || Boolean(env.WSL_DISTRO_NAME);
  const kind = !isWsl ? 'linux' : /wsl2|microsoft-standard/i.test(kernel) ? 'wsl2' : 'wsl1';
  return { kind, release: kernel, distro: env.WSL_DISTRO_NAME ?? '', ready: kind !== 'wsl1',
    message: kind === 'wsl2' ? '已检测到 WSL2，可以运行本地语音引擎。' : kind === 'linux'
      ? '当前为原生 Linux，可直接运行；Windows 用户请双击 launchers/start-windows.cmd 检查 WSL2。'
      : '当前发行版是 WSL1。请在 Windows PowerShell 运行 wsl --set-version <发行版名称> 2，完成后重新启动。' };
}

export const gptRequiredFiles = [
  ['GPT_SoVITS/pretrained_models/gsv-v2final-pretrained/s1bert25hz-5kh-longer-epoch=12-step=369668.ckpt', 1024 * 1024],
  ['GPT_SoVITS/pretrained_models/gsv-v2final-pretrained/s2G2333k.pth', 1024 * 1024],
  ['GPT_SoVITS/pretrained_models/gsv-v2final-pretrained/s2D2333k.pth', 1024 * 1024],
  ['GPT_SoVITS/pretrained_models/chinese-roberta-wwm-ext-large/pytorch_model.bin', 1024 * 1024],
  ['GPT_SoVITS/pretrained_models/chinese-roberta-wwm-ext-large/config.json', 1],
  ['GPT_SoVITS/pretrained_models/chinese-roberta-wwm-ext-large/tokenizer.json', 1],
  ['GPT_SoVITS/pretrained_models/chinese-hubert-base/pytorch_model.bin', 1024 * 1024],
  ['GPT_SoVITS/pretrained_models/chinese-hubert-base/config.json', 1],
  ['GPT_SoVITS/pretrained_models/fast_langdetect/lid.176.bin', 1024 * 1024],
  ['GPT_SoVITS/text/G2PWModel/g2pW.onnx', 1024 * 1024],
  ['GPT_SoVITS/text/G2PWModel/config.py', 1],
  ['GPT_SoVITS/text/G2PWModel/POLYPHONIC_CHARS.txt', 1],
  ['GPT_SoVITS/text/G2PWModel/MONOPHONIC_CHARS.txt', 1],
  ['GPT_SoVITS/text/G2PWModel/bopomofo_to_pinyin_wo_tune_dict.json', 1],
  ['GPT_SoVITS/text/G2PWModel/char_bopomofo_dict.json', 1],
];

export async function inspectGptModels(root, modelId = 'gpt-sovits-v2') {
  const missing = [];
  const weights = {
    'gpt-sovits-v2': gptRequiredFiles.slice(0, 3),
    'gpt-sovits-v2pro': ['s1v3.ckpt', 'v2Pro/s2Gv2Pro.pth', 'v2Pro/s2Dv2Pro.pth', 'sv/pretrained_eres2netv2w24s4ep4.ckpt'],
    'gpt-sovits-v2proplus': ['s1v3.ckpt', 'v2Pro/s2Gv2ProPlus.pth', 'v2Pro/s2Dv2ProPlus.pth', 'sv/pretrained_eres2netv2w24s4ep4.ckpt'],
    'gpt-sovits-v3': ['s1v3.ckpt', 's2Gv3.pth', 'models--nvidia--bigvgan_v2_24khz_100band_256x/bigvgan_generator.pt', 'models--nvidia--bigvgan_v2_24khz_100band_256x/config.json'],
    'gpt-sovits-v4': ['s1v3.ckpt', 'gsv-v4-pretrained/s2Gv4.pth', 'gsv-v4-pretrained/vocoder.pth'],
  }[modelId];
  if (!weights) return { ready: false, missing: ['未适配的模型版本'] };
  const required = [...gptRequiredFiles.slice(3), ...weights.map(name => Array.isArray(name) ? name
    : [`GPT_SoVITS/pretrained_models/${name}`, name.endsWith('.json') ? 1 : 1024 * 1024])];
  for (const [relative, minimum] of required) {
    try { const info = await stat(join(root, relative)); if (!info.isFile() || info.size < minimum) missing.push(relative); }
    catch { missing.push(relative); }
  }
  return { ready: missing.length === 0, missing };
}

export async function readSelection(root) {
  try {
    const value = JSON.parse(await readFile(join(root, 'config/local/model-selection.json'), 'utf8'));
    return typeof value.path === 'string' && value.path.startsWith('/') && typeof value.id === 'string' ? value : null;
  } catch { return null; }
}
