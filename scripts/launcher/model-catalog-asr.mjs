/** Local ASR weights; file lists checked against publisher metadata on 2026-09-21. */
const whisperFiles = ['model.bin', 'config.json', 'tokenizer.json', 'preprocessor_config.json'];
const qwenFiles = ['config.json', 'generation_config.json', 'preprocessor_config.json', 'tokenizer_config.json', 'chat_template.json', 'merges.txt', 'vocab.json'];
const entry = (id, name, repo, languages, description, license, homepage, files, ready = false) => ({
  id, name, purpose: 'asr', languages, description, license, homepage,
  source_url: `https://huggingface.co/${repo}`, compatibility: ready ? 'ready' : 'download_only',
  note: ready ? '用于训练录音自动提取文本。使用训练 Python 中的 faster-whisper，CPU INT8 运行，识别完成后卸载。'
    : '可按需下载；尚未接入本项目自动转写，下载后不能直接选择使用。',
  sources: [{ repo, prefix: '', include: ['README.md', ...files] }],
  markers: files,
});

export const ASR_MODELS = [
  entry('faster-whisper-large-v3-turbo', 'Whisper large-v3-turbo', 'mobiuslabsgmbh/faster-whisper-large-v3-turbo',
    '中文 / 英文 / 日文 / 韩文 / 粤语等多语言', '推荐：多语言训练录音转写，兼顾速度与识别质量。', 'MIT',
    'https://huggingface.co/openai/whisper-large-v3-turbo', [...whisperFiles, 'vocabulary.json'], true),
  entry('faster-whisper-large-v3', 'Whisper large-v3', 'Systran/faster-whisper-large-v3',
    '中文 / 英文 / 日文 / 韩文 / 粤语等多语言', '完整版 Whisper，适合能接受较长识别等待的场景。', 'MIT',
    'https://github.com/SYSTRAN/faster-whisper', [...whisperFiles, 'vocabulary.json'], true),
  entry('sensevoice-small', 'SenseVoiceSmall', 'FunAudioLLM/SenseVoiceSmall',
    '中文 / 粤语 / 英文 / 日文 / 韩文', '适合中文短录音的轻量语音理解模型，兼有情绪和声音事件识别。', '见官方模型许可',
    'https://github.com/FunAudioLLM/SenseVoice', ['model.pt', 'config.yaml', 'configuration.json', 'am.mvn', 'chn_jpn_yue_eng_ko_spectok.bpe.model']),
  ...['0.6', '1.7'].map(size => entry(`qwen3-asr-${size}b`, `Qwen3-ASR ${size}B`, `Qwen/Qwen3-ASR-${size}B`,
    '30 种语言 / 22 种中文方言', size === '0.6' ? '偏重效率，适合评估方言和多语言训练录音。' : '偏重识别质量，运行资源需求更高。', 'Apache-2.0',
    'https://github.com/QwenLM/Qwen3-ASR', [...qwenFiles, ...(size === '0.6' ? ['model.safetensors']
      : ['model-00001-of-00002.safetensors', 'model-00002-of-00002.safetensors', 'model.safetensors.index.json'])])),
  entry('paraformer-zh', 'Paraformer-zh', 'funasr/paraformer-zh', '普通话 / 英文',
    '面向普通话和中英混合录音的非自回归识别模型。', 'Apache-2.0', 'https://github.com/modelscope/FunASR',
    ['model.pt', 'config.yaml', 'configuration.json', 'am.mvn', 'seg_dict', 'tokens.json']),
];
