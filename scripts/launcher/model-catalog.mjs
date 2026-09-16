/** Curated, data-only voice model downloads. Verified against official HF metadata on 2026-09-16.
 * include entries match one exact file or a directory prefix ending in /; no globs or executable installers.
 * markers cover selected weights and loading configuration, not proof that an engine can run.
 * GPT G2PWModel.zip is extracted separately by the launcher into GPT_SoVITS/text/G2PWModel.
 */
export const MODEL_CATALOG = [
  {
    "id": "gpt-sovits-v2",
    "name": "GPT-SoVITS v2",
    "languages": "中文 / 英文 / 日文 / 韩文 / 粤语",
    "description": "已接入的参考音频克隆引擎，适合本项目语音播报。",
    "license": "MIT；附属模型遵循各自许可",
    "homepage": "https://github.com/RVC-Boss/GPT-SoVITS",
    "source_url": "https://huggingface.co/lj1995/GPT-SoVITS",
    "compatibility": "ready",
    "note": "下载含 v2 权重、中文编码器、语言识别和 G2PW 资源；仍需已有 GPT-SoVITS 引擎代码及 Python 依赖，通过环境检查后可选择启用。",
    "sources": [
      {
        "repo": "lj1995/GPT-SoVITS",
        "prefix": "GPT_SoVITS/pretrained_models",
        "include": [
          "README.md",
          "chinese-hubert-base/",
          "chinese-roberta-wwm-ext-large/",
          "gsv-v2final-pretrained/"
        ]
      },
      {
        "repo": "XXXXRT/GPT-SoVITS-Pretrained",
        "prefix": "GPT_SoVITS",
        "include": [
          "pretrained_models/fast_langdetect/lid.176.bin"
        ]
      },
      {
        "repo": "XXXXRT/GPT-SoVITS-Pretrained",
        "prefix": "",
        "include": [
          "G2PWModel.zip"
        ]
      }
    ],
    "markers": [
      "GPT_SoVITS/pretrained_models/chinese-hubert-base/config.json",
      "GPT_SoVITS/pretrained_models/chinese-hubert-base/preprocessor_config.json",
      "GPT_SoVITS/pretrained_models/chinese-hubert-base/pytorch_model.bin",
      "GPT_SoVITS/pretrained_models/chinese-roberta-wwm-ext-large/config.json",
      "GPT_SoVITS/pretrained_models/chinese-roberta-wwm-ext-large/pytorch_model.bin",
      "GPT_SoVITS/pretrained_models/chinese-roberta-wwm-ext-large/tokenizer.json",
      "GPT_SoVITS/pretrained_models/gsv-v2final-pretrained/s1bert25hz-5kh-longer-epoch=12-step=369668.ckpt",
      "GPT_SoVITS/pretrained_models/gsv-v2final-pretrained/s2D2333k.pth",
      "GPT_SoVITS/pretrained_models/gsv-v2final-pretrained/s2G2333k.pth",
      "GPT_SoVITS/pretrained_models/fast_langdetect/lid.176.bin",
      "G2PWModel.zip"
    ]
  },
  {
    "id": "gpt-sovits-v3",
    "name": "GPT-SoVITS v3",
    "languages": "中文 / 英文 / 日文 / 韩文 / 粤语",
    "description": "GPT-SoVITS 第三版，提供更丰富的参考音色表现。",
    "license": "MIT；附属模型遵循各自许可",
    "homepage": "https://github.com/RVC-Boss/GPT-SoVITS",
    "source_url": "https://huggingface.co/lj1995/GPT-SoVITS",
    "compatibility": "download_only",
    "note": "可下载模型文件；本项目尚未接入此引擎，需按官方说明另行安装并适配语音接口。",
    "sources": [
      {
        "repo": "lj1995/GPT-SoVITS",
        "prefix": "GPT_SoVITS/pretrained_models",
        "include": [
          "README.md",
          "chinese-hubert-base/",
          "chinese-roberta-wwm-ext-large/",
          "s1v3.ckpt",
          "s2Gv3.pth",
          "models--nvidia--bigvgan_v2_24khz_100band_256x/"
        ]
      },
      {
        "repo": "XXXXRT/GPT-SoVITS-Pretrained",
        "prefix": "GPT_SoVITS",
        "include": [
          "pretrained_models/fast_langdetect/lid.176.bin"
        ]
      },
      {
        "repo": "XXXXRT/GPT-SoVITS-Pretrained",
        "prefix": "",
        "include": [
          "G2PWModel.zip"
        ]
      }
    ],
    "markers": [
      "GPT_SoVITS/pretrained_models/chinese-hubert-base/config.json",
      "GPT_SoVITS/pretrained_models/chinese-hubert-base/preprocessor_config.json",
      "GPT_SoVITS/pretrained_models/chinese-hubert-base/pytorch_model.bin",
      "GPT_SoVITS/pretrained_models/chinese-roberta-wwm-ext-large/config.json",
      "GPT_SoVITS/pretrained_models/chinese-roberta-wwm-ext-large/pytorch_model.bin",
      "GPT_SoVITS/pretrained_models/chinese-roberta-wwm-ext-large/tokenizer.json",
      "GPT_SoVITS/pretrained_models/s1v3.ckpt",
      "GPT_SoVITS/pretrained_models/s2Gv3.pth",
      "GPT_SoVITS/pretrained_models/models--nvidia--bigvgan_v2_24khz_100band_256x/bigvgan_generator.pt",
      "GPT_SoVITS/pretrained_models/models--nvidia--bigvgan_v2_24khz_100band_256x/config.json",
      "GPT_SoVITS/pretrained_models/fast_langdetect/lid.176.bin",
      "G2PWModel.zip"
    ]
  },
  {
    "id": "gpt-sovits-v4",
    "name": "GPT-SoVITS v4",
    "languages": "中文 / 英文 / 日文 / 韩文 / 粤语",
    "description": "GPT-SoVITS 第四版，支持原生 48 kHz 输出。",
    "license": "MIT；附属模型遵循各自许可",
    "homepage": "https://github.com/RVC-Boss/GPT-SoVITS",
    "source_url": "https://huggingface.co/lj1995/GPT-SoVITS",
    "compatibility": "download_only",
    "note": "可下载模型文件；本项目尚未接入此引擎，需按官方说明另行安装并适配语音接口。",
    "sources": [
      {
        "repo": "lj1995/GPT-SoVITS",
        "prefix": "GPT_SoVITS/pretrained_models",
        "include": [
          "README.md",
          "chinese-hubert-base/",
          "chinese-roberta-wwm-ext-large/",
          "s1v3.ckpt",
          "gsv-v4-pretrained/"
        ]
      },
      {
        "repo": "XXXXRT/GPT-SoVITS-Pretrained",
        "prefix": "GPT_SoVITS",
        "include": [
          "pretrained_models/fast_langdetect/lid.176.bin"
        ]
      },
      {
        "repo": "XXXXRT/GPT-SoVITS-Pretrained",
        "prefix": "",
        "include": [
          "G2PWModel.zip"
        ]
      }
    ],
    "markers": [
      "GPT_SoVITS/pretrained_models/chinese-hubert-base/config.json",
      "GPT_SoVITS/pretrained_models/chinese-hubert-base/preprocessor_config.json",
      "GPT_SoVITS/pretrained_models/chinese-hubert-base/pytorch_model.bin",
      "GPT_SoVITS/pretrained_models/chinese-roberta-wwm-ext-large/config.json",
      "GPT_SoVITS/pretrained_models/chinese-roberta-wwm-ext-large/pytorch_model.bin",
      "GPT_SoVITS/pretrained_models/chinese-roberta-wwm-ext-large/tokenizer.json",
      "GPT_SoVITS/pretrained_models/s1v3.ckpt",
      "GPT_SoVITS/pretrained_models/gsv-v4-pretrained/s2Gv4.pth",
      "GPT_SoVITS/pretrained_models/gsv-v4-pretrained/vocoder.pth",
      "GPT_SoVITS/pretrained_models/fast_langdetect/lid.176.bin",
      "G2PWModel.zip"
    ]
  },
  {
    "id": "cosyvoice-2",
    "name": "CosyVoice 2 · 0.5B",
    "languages": "中文 / 英文 / 日文 / 韩文 / 方言",
    "description": "支持参考音频克隆与流式合成的多语言语音模型。",
    "license": "Apache-2.0",
    "homepage": "https://github.com/QwenAudio/CosyVoice",
    "source_url": "https://huggingface.co/FunAudioLLM/CosyVoice2-0.5B",
    "compatibility": "download_only",
    "note": "下载常规推理权重；不包含可选 TensorRT / 批处理导出和 RL 变体。本项目尚未适配此引擎。",
    "sources": [
      {
        "repo": "FunAudioLLM/CosyVoice2-0.5B",
        "prefix": "",
        "include": [
          "README.md",
          "CosyVoice-BlankEN/",
          "campplus.onnx",
          "config.json",
          "configuration.json",
          "cosyvoice2.yaml",
          "flow.pt",
          "hift.pt",
          "llm.pt",
          "speech_tokenizer_v2.onnx"
        ]
      }
    ],
    "markers": [
      "CosyVoice-BlankEN/config.json",
      "CosyVoice-BlankEN/generation_config.json",
      "CosyVoice-BlankEN/merges.txt",
      "CosyVoice-BlankEN/model.safetensors",
      "CosyVoice-BlankEN/tokenizer_config.json",
      "CosyVoice-BlankEN/vocab.json",
      "campplus.onnx",
      "config.json",
      "configuration.json",
      "cosyvoice2.yaml",
      "flow.pt",
      "hift.pt",
      "llm.pt",
      "speech_tokenizer_v2.onnx"
    ]
  },
  {
    "id": "cosyvoice-3",
    "name": "CosyVoice 3 · 0.5B",
    "languages": "中英日韩等 9 种语言 / 中文方言",
    "description": "支持参考音频克隆与流式合成的多语言语音模型。",
    "license": "Apache-2.0",
    "homepage": "https://github.com/QwenAudio/CosyVoice",
    "source_url": "https://huggingface.co/FunAudioLLM/Fun-CosyVoice3-0.5B-2512",
    "compatibility": "download_only",
    "note": "下载常规推理权重；不包含可选 TensorRT / 批处理导出和 RL 变体。本项目尚未适配此引擎。",
    "sources": [
      {
        "repo": "FunAudioLLM/Fun-CosyVoice3-0.5B-2512",
        "prefix": "",
        "include": [
          "README.md",
          "CosyVoice-BlankEN/",
          "campplus.onnx",
          "config.json",
          "configuration.json",
          "cosyvoice3.yaml",
          "flow.pt",
          "hift.pt",
          "llm.pt",
          "speech_tokenizer_v3.onnx"
        ]
      }
    ],
    "markers": [
      "CosyVoice-BlankEN/config.json",
      "CosyVoice-BlankEN/generation_config.json",
      "CosyVoice-BlankEN/merges.txt",
      "CosyVoice-BlankEN/model.safetensors",
      "CosyVoice-BlankEN/tokenizer_config.json",
      "CosyVoice-BlankEN/vocab.json",
      "campplus.onnx",
      "config.json",
      "configuration.json",
      "cosyvoice3.yaml",
      "flow.pt",
      "hift.pt",
      "llm.pt",
      "speech_tokenizer_v3.onnx"
    ]
  },
  {
    "id": "indextts-2",
    "name": "IndexTTS 2",
    "languages": "中文 / 英文",
    "description": "强调情绪与时长控制的零样本语音合成模型。",
    "license": "bilibili Model Use License",
    "homepage": "https://github.com/index-tts/index-tts",
    "source_url": "https://huggingface.co/IndexTeam/IndexTTS-2",
    "compatibility": "download_only",
    "note": "下载主检查点和情绪模型；运行时另有编码器 / 声码器依赖。遵循官方模型使用协议，本项目尚未适配。",
    "sources": [
      {
        "repo": "IndexTeam/IndexTTS-2",
        "prefix": "",
        "include": [
          "README.md",
          "config.yaml",
          "feat1.pt",
          "feat2.pt",
          "gpt.pth",
          "qwen0.6bemo4-merge/",
          "s2mel.pth",
          "wav2vec2bert_stats.pt",
          "bpe.model",
          "LICENSE.txt",
          "LICENSE_ZH.txt"
        ]
      }
    ],
    "markers": [
      "config.yaml",
      "feat1.pt",
      "feat2.pt",
      "gpt.pth",
      "qwen0.6bemo4-merge/added_tokens.json",
      "qwen0.6bemo4-merge/chat_template.jinja",
      "qwen0.6bemo4-merge/config.json",
      "qwen0.6bemo4-merge/generation_config.json",
      "qwen0.6bemo4-merge/merges.txt",
      "qwen0.6bemo4-merge/model.safetensors",
      "qwen0.6bemo4-merge/special_tokens_map.json",
      "qwen0.6bemo4-merge/tokenizer.json",
      "qwen0.6bemo4-merge/tokenizer_config.json",
      "qwen0.6bemo4-merge/vocab.json",
      "s2mel.pth",
      "wav2vec2bert_stats.pt",
      "bpe.model"
    ]
  },
  {
    "id": "indextts-2-5",
    "name": "IndexTTS 2.5",
    "languages": "中文 / 英文 / 日文 / 西班牙文",
    "description": "强调情绪与时长控制的零样本语音合成模型。",
    "license": "bilibili Model Use License",
    "homepage": "https://github.com/index-tts/index-tts",
    "source_url": "https://huggingface.co/IndexTeam/IndexTTS-2.5",
    "compatibility": "download_only",
    "note": "下载主检查点和情绪模型；运行时另有编码器 / 声码器依赖。遵循官方模型使用协议，本项目尚未适配。",
    "sources": [
      {
        "repo": "IndexTeam/IndexTTS-2.5",
        "prefix": "",
        "include": [
          "README.md",
          "config.yaml",
          "feat1.pt",
          "feat2.pt",
          "gpt.pth",
          "qwen0.6bemo4-merge/",
          "s2mel.pth",
          "wav2vec2bert_stats.pt",
          "codec.pth",
          "multilingual_zh_ja_yue_char_del.tiktoken",
          "LICENSE"
        ]
      }
    ],
    "markers": [
      "config.yaml",
      "feat1.pt",
      "feat2.pt",
      "gpt.pth",
      "qwen0.6bemo4-merge/added_tokens.json",
      "qwen0.6bemo4-merge/chat_template.jinja",
      "qwen0.6bemo4-merge/config.json",
      "qwen0.6bemo4-merge/generation_config.json",
      "qwen0.6bemo4-merge/merges.txt",
      "qwen0.6bemo4-merge/model.safetensors",
      "qwen0.6bemo4-merge/special_tokens_map.json",
      "qwen0.6bemo4-merge/tokenizer.json",
      "qwen0.6bemo4-merge/tokenizer_config.json",
      "qwen0.6bemo4-merge/vocab.json",
      "s2mel.pth",
      "wav2vec2bert_stats.pt",
      "codec.pth",
      "multilingual_zh_ja_yue_char_del.tiktoken"
    ]
  },
  {
    "id": "f5-tts-v1",
    "name": "F5-TTS v1 Base",
    "languages": "中文 / 英文",
    "description": "以流匹配生成语音，支持参考音频克隆。",
    "license": "CC-BY-NC-4.0（非商业）；Vocos 为 MIT",
    "homepage": "https://github.com/SWivid/F5-TTS",
    "source_url": "https://huggingface.co/SWivid/F5-TTS",
    "compatibility": "download_only",
    "note": "可下载模型文件；本项目尚未接入此引擎，需按官方说明另行安装并适配语音接口。",
    "sources": [
      {
        "repo": "SWivid/F5-TTS",
        "prefix": "",
        "include": [
          "README.md",
          "F5TTS_v1_Base/"
        ]
      },
      {
        "repo": "charactr/vocos-mel-24khz",
        "prefix": "vocos-mel-24khz",
        "include": [
          "README.md",
          "config.yaml",
          "pytorch_model.bin"
        ]
      }
    ],
    "markers": [
      "F5TTS_v1_Base/model_1250000.safetensors",
      "F5TTS_v1_Base/vocab.txt",
      "vocos-mel-24khz/config.yaml",
      "vocos-mel-24khz/pytorch_model.bin"
    ]
  },
  {
    "id": "kokoro-82m",
    "name": "Kokoro · 82M",
    "languages": "中英日法西等多语言",
    "description": "体积较小的多语言语音模型，附带预设音色文件。",
    "license": "Apache-2.0",
    "homepage": "https://github.com/hexgrad/kokoro",
    "source_url": "https://huggingface.co/hexgrad/Kokoro-82M",
    "compatibility": "download_only",
    "note": "下载模型和预设音色；运行仍需对应语言的发音处理依赖，本项目尚未适配。",
    "sources": [
      {
        "repo": "hexgrad/Kokoro-82M",
        "prefix": "",
        "include": [
          "README.md",
          "VOICES.md",
          "config.json",
          "kokoro-v1_0.pth",
          "voices/"
        ]
      }
    ],
    "markers": [
      "config.json",
      "kokoro-v1_0.pth",
      "voices/af_alloy.pt",
      "voices/af_aoede.pt",
      "voices/af_bella.pt",
      "voices/af_heart.pt",
      "voices/af_jessica.pt",
      "voices/af_kore.pt",
      "voices/af_nicole.pt",
      "voices/af_nova.pt",
      "voices/af_river.pt",
      "voices/af_sarah.pt",
      "voices/af_sky.pt",
      "voices/am_adam.pt",
      "voices/am_echo.pt",
      "voices/am_eric.pt",
      "voices/am_fenrir.pt",
      "voices/am_liam.pt",
      "voices/am_michael.pt",
      "voices/am_onyx.pt",
      "voices/am_puck.pt",
      "voices/am_santa.pt",
      "voices/bf_alice.pt",
      "voices/bf_emma.pt",
      "voices/bf_isabella.pt",
      "voices/bf_lily.pt",
      "voices/bm_daniel.pt",
      "voices/bm_fable.pt",
      "voices/bm_george.pt",
      "voices/bm_lewis.pt",
      "voices/ef_dora.pt",
      "voices/em_alex.pt",
      "voices/em_santa.pt",
      "voices/ff_siwis.pt",
      "voices/hf_alpha.pt",
      "voices/hf_beta.pt",
      "voices/hm_omega.pt",
      "voices/hm_psi.pt",
      "voices/if_sara.pt",
      "voices/im_nicola.pt",
      "voices/jf_alpha.pt",
      "voices/jf_gongitsune.pt",
      "voices/jf_nezumi.pt",
      "voices/jf_tebukuro.pt",
      "voices/jm_kumo.pt",
      "voices/pf_dora.pt",
      "voices/pm_alex.pt",
      "voices/pm_santa.pt",
      "voices/zf_xiaobei.pt",
      "voices/zf_xiaoni.pt",
      "voices/zf_xiaoxiao.pt",
      "voices/zf_xiaoyi.pt",
      "voices/zm_yunjian.pt",
      "voices/zm_yunxi.pt",
      "voices/zm_yunxia.pt",
      "voices/zm_yunyang.pt"
    ]
  },
  {
    "id": "fish-speech-1-5",
    "name": "Fish Speech 1.5",
    "languages": "中英日韩等多语言",
    "description": "公开可下载的 Fish Speech 语音生成模型。",
    "license": "CC-BY-NC-SA-4.0（非商业）",
    "homepage": "https://github.com/fishaudio/fish-speech",
    "source_url": "https://huggingface.co/fishaudio/fish-speech-1.5",
    "compatibility": "download_only",
    "note": "该条目为公开的 1.5 版；S1-mini 需申请仓库访问，不作为匿名一键下载选项。此引擎尚未适配。",
    "sources": [
      {
        "repo": "fishaudio/fish-speech-1.5",
        "prefix": "",
        "include": [
          "README.md",
          "config.json",
          "firefly-gan-vq-fsq-8x1024-21hz-generator.pth",
          "model.pth",
          "special_tokens.json",
          "tokenizer.tiktoken"
        ]
      }
    ],
    "markers": [
      "config.json",
      "firefly-gan-vq-fsq-8x1024-21hz-generator.pth",
      "model.pth",
      "special_tokens.json",
      "tokenizer.tiktoken"
    ]
  },
  {
    "id": "chattts",
    "name": "ChatTTS",
    "languages": "中文 / 英文",
    "description": "面向日常对话、带停顿和口语表达的语音模型。",
    "license": "CC-BY-NC-4.0（非商业）",
    "homepage": "https://github.com/2noise/ChatTTS",
    "source_url": "https://huggingface.co/2Noise/ChatTTS",
    "compatibility": "download_only",
    "note": "可下载模型文件；本项目尚未接入此引擎，需按官方说明另行安装并适配语音接口。",
    "sources": [
      {
        "repo": "2Noise/ChatTTS",
        "prefix": "",
        "include": [
          "README.md",
          "asset/DVAE.safetensors",
          "asset/Decoder.safetensors",
          "asset/Embed.safetensors",
          "asset/Vocos.safetensors",
          "asset/gpt/",
          "asset/tokenizer/",
          "asset/spk_stat.pt",
          "config/"
        ]
      }
    ],
    "markers": [
      "asset/DVAE.safetensors",
      "asset/Decoder.safetensors",
      "asset/Embed.safetensors",
      "asset/Vocos.safetensors",
      "asset/gpt/config.json",
      "asset/gpt/model.safetensors",
      "asset/tokenizer/special_tokens_map.json",
      "asset/tokenizer/tokenizer.json",
      "asset/tokenizer/tokenizer_config.json",
      "asset/spk_stat.pt",
      "config/decoder.yaml",
      "config/dvae.yaml",
      "config/gpt.yaml",
      "config/path.yaml",
      "config/vocos.yaml"
    ]
  },
  {
    "id": "xtts-v2",
    "name": "XTTS v2",
    "languages": "中英日韩等 17 种语言",
    "description": "支持跨语言音色克隆的多语言语音合成模型。",
    "license": "Coqui Public Model License",
    "homepage": "https://github.com/coqui-ai/TTS",
    "source_url": "https://huggingface.co/coqui/XTTS-v2",
    "compatibility": "download_only",
    "note": "模型采用专用 CPML 协议，请查看官方许可；本项目尚未接入 XTTS 引擎。",
    "sources": [
      {
        "repo": "coqui/XTTS-v2",
        "prefix": "",
        "include": [
          "README.md",
          "LICENSE.txt",
          "config.json",
          "dvae.pth",
          "mel_stats.pth",
          "model.pth",
          "speakers_xtts.pth",
          "vocab.json"
        ]
      }
    ],
    "markers": [
      "config.json",
      "dvae.pth",
      "mel_stats.pth",
      "model.pth",
      "speakers_xtts.pth",
      "vocab.json"
    ]
  },
  {
    "id": "melotts-chinese",
    "name": "MeloTTS · 中文",
    "languages": "中文 / 中英混合",
    "description": "较轻量的中文语音合成模型。",
    "license": "MIT",
    "homepage": "https://github.com/myshell-ai/MeloTTS",
    "source_url": "https://huggingface.co/myshell-ai/MeloTTS-Chinese",
    "compatibility": "download_only",
    "note": "此包为中文检查点；运行另需文本编码器与字典资源，本项目尚未适配。",
    "sources": [
      {
        "repo": "myshell-ai/MeloTTS-Chinese",
        "prefix": "",
        "include": [
          "README.md",
          "checkpoint.pth",
          "config.json"
        ]
      }
    ],
    "markers": [
      "checkpoint.pth",
      "config.json"
    ]
  },
  {
    "id": "openvoice-v2",
    "name": "OpenVoice v2",
    "languages": "中英日韩法西等语言",
    "description": "音色转换与跨语言克隆组件，可与基础 TTS 配合。",
    "license": "MIT",
    "homepage": "https://github.com/myshell-ai/OpenVoice",
    "source_url": "https://huggingface.co/myshell-ai/OpenVoiceV2",
    "compatibility": "download_only",
    "note": "下载音色转换模型和说话人嵌入；还需 MeloTTS 等基础语音引擎。本项目尚未适配。",
    "sources": [
      {
        "repo": "myshell-ai/OpenVoiceV2",
        "prefix": "",
        "include": [
          "README.md",
          "converter/",
          "base_speakers/ses/"
        ]
      }
    ],
    "markers": [
      "converter/checkpoint.pth",
      "converter/config.json",
      "base_speakers/ses/en-au.pth",
      "base_speakers/ses/en-br.pth",
      "base_speakers/ses/en-default.pth",
      "base_speakers/ses/en-india.pth",
      "base_speakers/ses/en-newest.pth",
      "base_speakers/ses/en-us.pth",
      "base_speakers/ses/es.pth",
      "base_speakers/ses/fr.pth",
      "base_speakers/ses/jp.pth",
      "base_speakers/ses/kr.pth",
      "base_speakers/ses/zh.pth"
    ]
  },
  {
    "id": "piper-zh-huayan",
    "name": "Piper · 中文华严",
    "languages": "中文",
    "description": "ONNX 语音模型，适合探索 CPU 本地合成。",
    "license": "此音色 MODEL_CARD 未明确许可",
    "homepage": "https://github.com/OHF-Voice/piper1-gpl",
    "source_url": "https://huggingface.co/rhasspy/piper-voices",
    "compatibility": "download_only",
    "note": "仅下载华严中文 medium 音色，不下载整个多语言仓库；音色许可与 Piper 引擎许可分别判断。本项目尚未适配。",
    "sources": [
      {
        "repo": "rhasspy/piper-voices",
        "prefix": "",
        "include": [
          "README.md",
          "zh/zh_CN/huayan/medium/MODEL_CARD",
          "zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx",
          "zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx.json"
        ]
      }
    ],
    "markers": [
      "zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx",
      "zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx.json"
    ]
  },
  {
    "id": "chatterbox-multilingual",
    "name": "Chatterbox Multilingual",
    "languages": "中英日韩等 23 种语言",
    "description": "带情感控制与参考音色克隆的多语言语音模型。",
    "license": "MIT",
    "homepage": "https://github.com/resemble-ai/chatterbox",
    "source_url": "https://huggingface.co/ResembleAI/chatterbox",
    "compatibility": "download_only",
    "note": "可下载模型文件；本项目尚未接入此引擎，需按官方说明另行安装并适配语音接口。",
    "sources": [
      {
        "repo": "ResembleAI/chatterbox",
        "prefix": "",
        "include": [
          "README.md",
          "conds.pt",
          "s3gen.pt",
          "t3_mtl23ls_v2.safetensors",
          "ve.pt",
          "mtl_tokenizer.json",
          "Cangjie5_TC.json",
          "grapheme_mtl_merged_expanded_v1.json"
        ]
      }
    ],
    "markers": [
      "conds.pt",
      "s3gen.pt",
      "t3_mtl23ls_v2.safetensors",
      "ve.pt",
      "mtl_tokenizer.json",
      "Cangjie5_TC.json",
      "grapheme_mtl_merged_expanded_v1.json"
    ]
  },
  {
    "id": "chatterbox-turbo",
    "name": "Chatterbox Turbo",
    "languages": "英文",
    "description": "面向低延迟生成、支持语气标签的 Chatterbox 变体。",
    "license": "MIT",
    "homepage": "https://github.com/resemble-ai/chatterbox",
    "source_url": "https://huggingface.co/ResembleAI/chatterbox-turbo",
    "compatibility": "download_only",
    "note": "可下载模型文件；本项目尚未接入此引擎，需按官方说明另行安装并适配语音接口。",
    "sources": [
      {
        "repo": "ResembleAI/chatterbox-turbo",
        "prefix": "",
        "include": [
          "README.md",
          "added_tokens.json",
          "conds.pt",
          "merges.txt",
          "s3gen_meanflow.safetensors",
          "special_tokens_map.json",
          "t3_turbo_v1.safetensors",
          "t3_turbo_v1.yaml",
          "tokenizer_config.json",
          "ve.safetensors",
          "vocab.json"
        ]
      }
    ],
    "markers": [
      "added_tokens.json",
      "conds.pt",
      "merges.txt",
      "s3gen_meanflow.safetensors",
      "special_tokens_map.json",
      "t3_turbo_v1.safetensors",
      "t3_turbo_v1.yaml",
      "tokenizer_config.json",
      "ve.safetensors",
      "vocab.json"
    ]
  },
  {
    "id": "qwen3-tts-0-6b-base",
    "name": "Qwen3-TTS · 0.6B Base",
    "languages": "中英日韩德法俄葡西意 10 种语言",
    "description": "参考音频克隆基础模型。",
    "license": "Apache-2.0",
    "homepage": "https://github.com/QwenLM/Qwen3-TTS",
    "source_url": "https://huggingface.co/Qwen/Qwen3-TTS-12Hz-0.6B-Base",
    "compatibility": "download_only",
    "note": "可下载模型文件；本项目尚未接入此引擎，需按官方说明另行安装并适配语音接口。",
    "sources": [
      {
        "repo": "Qwen/Qwen3-TTS-12Hz-0.6B-Base",
        "prefix": "",
        "include": [
          "README.md",
          "config.json",
          "generation_config.json",
          "merges.txt",
          "model.safetensors",
          "preprocessor_config.json",
          "speech_tokenizer/",
          "tokenizer_config.json",
          "vocab.json"
        ]
      }
    ],
    "markers": [
      "config.json",
      "generation_config.json",
      "merges.txt",
      "model.safetensors",
      "preprocessor_config.json",
      "speech_tokenizer/config.json",
      "speech_tokenizer/configuration.json",
      "speech_tokenizer/model.safetensors",
      "speech_tokenizer/preprocessor_config.json",
      "tokenizer_config.json",
      "vocab.json"
    ]
  },
  {
    "id": "qwen3-tts-1-7b-customvoice",
    "name": "Qwen3-TTS · 1.7B CustomVoice",
    "languages": "中英日韩德法俄葡西意 10 种语言",
    "description": "预设说话人与自然语言风格控制模型。",
    "license": "Apache-2.0",
    "homepage": "https://github.com/QwenLM/Qwen3-TTS",
    "source_url": "https://huggingface.co/Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice",
    "compatibility": "download_only",
    "note": "可下载模型文件；本项目尚未接入此引擎，需按官方说明另行安装并适配语音接口。",
    "sources": [
      {
        "repo": "Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice",
        "prefix": "",
        "include": [
          "README.md",
          "config.json",
          "generation_config.json",
          "merges.txt",
          "model.safetensors",
          "preprocessor_config.json",
          "speech_tokenizer/",
          "tokenizer_config.json",
          "vocab.json"
        ]
      }
    ],
    "markers": [
      "config.json",
      "generation_config.json",
      "merges.txt",
      "model.safetensors",
      "preprocessor_config.json",
      "speech_tokenizer/config.json",
      "speech_tokenizer/configuration.json",
      "speech_tokenizer/model.safetensors",
      "speech_tokenizer/preprocessor_config.json",
      "tokenizer_config.json",
      "vocab.json"
    ]
  },
  {
    "id": "spark-tts",
    "name": "Spark-TTS · 0.5B",
    "languages": "中文 / 英文",
    "description": "支持音色克隆和属性控制的双语语音模型。",
    "license": "CC-BY-NC-SA-4.0（非商业）",
    "homepage": "https://github.com/SparkAudio/Spark-TTS",
    "source_url": "https://huggingface.co/SparkAudio/Spark-TTS-0.5B",
    "compatibility": "download_only",
    "note": "官方模型许可已改为 CC-BY-NC-SA-4.0，不能沿用旧版 Apache-2.0 印象；本项目尚未适配。",
    "sources": [
      {
        "repo": "SparkAudio/Spark-TTS-0.5B",
        "prefix": "",
        "include": [
          "README.md",
          "BiCodec/",
          "LLM/",
          "config.yaml",
          "wav2vec2-large-xlsr-53/"
        ]
      }
    ],
    "markers": [
      "BiCodec/config.yaml",
      "BiCodec/model.safetensors",
      "LLM/added_tokens.json",
      "LLM/config.json",
      "LLM/merges.txt",
      "LLM/model.safetensors",
      "LLM/special_tokens_map.json",
      "LLM/tokenizer.json",
      "LLM/tokenizer_config.json",
      "LLM/vocab.json",
      "config.yaml",
      "wav2vec2-large-xlsr-53/config.json",
      "wav2vec2-large-xlsr-53/preprocessor_config.json",
      "wav2vec2-large-xlsr-53/pytorch_model.bin"
    ]
  },
  {
    "id": "dia2-1b",
    "name": "Dia2 · 1B",
    "languages": "英文",
    "description": "面向实时对话和多说话人合成的语音模型。",
    "license": "Apache-2.0；附属资源各自许可",
    "homepage": "https://github.com/nari-labs/dia2",
    "source_url": "https://huggingface.co/nari-labs/Dia2-1B",
    "compatibility": "download_only",
    "note": "下载 Dia2 主模型与分词器；运行另需官方指定的 Mimi 编解码器。本项目尚未适配。",
    "sources": [
      {
        "repo": "nari-labs/Dia2-1B",
        "prefix": "",
        "include": [
          "README.md",
          "added_tokens.json",
          "config.json",
          "dia2_assets.json",
          "merges.txt",
          "model.safetensors",
          "special_tokens_map.json",
          "tokenizer.json",
          "tokenizer_config.json",
          "vocab.json"
        ]
      }
    ],
    "markers": [
      "added_tokens.json",
      "config.json",
      "dia2_assets.json",
      "merges.txt",
      "model.safetensors",
      "special_tokens_map.json",
      "tokenizer.json",
      "tokenizer_config.json",
      "vocab.json"
    ]
  }
];
