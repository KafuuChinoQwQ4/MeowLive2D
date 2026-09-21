import type { ModelLibrarySnapshot } from "@meowlive/contracts";

export function modelLibrarySnapshot(overrides: Partial<ModelLibrarySnapshot> = {}): ModelLibrarySnapshot {
  return {
    schema_version: 1,
    environment: { kind: "wsl2", release: "6.6.87.2-microsoft-standard-WSL2", distro: "Ubuntu", ready: true, message: "WSL2 环境可用" },
    runtime: { engine_root: "./data/engines/GPT-SoVITS", python_path: "~/environments/gpt-sovits/bin/python", ready: true, message: "GPT-SoVITS 引擎已准备好" },
    scan_roots: ["./data/engines", "./data/models"],
    installed: [{ id: "local-gpt", model_id: "gpt-sovits-v2", purpose: "tts", name: "GPT-SoVITS v2", path: "./data/engines/GPT-SoVITS", ready: true, selected: true, message: "模型完整，可以使用" }],
    catalog: [{ id: "gpt-sovits-v2", purpose: "tts", name: "GPT-SoVITS v2", languages: "中文、英文、日文等", description: "少样本声音克隆", license: "MIT", homepage: "https://github.com/RVC-Boss/GPT-SoVITS", source_url: "https://huggingface.co/lj1995/GPT-SoVITS", compatibility: "ready", note: "已接入本项目" },
      ...Array.from({ length: 6 }, (_, i) => ({ id: `model-${i}`, purpose: "tts", name: `语音模型 ${i}`, languages: "中文、英文", description: "公开语音模型", license: "见官方许可", homepage: "https://github.com/example/model", source_url: "https://huggingface.co/example/model", compatibility: "download_only" as const, note: "下载后需要安装引擎与接口适配" }))],
    downloads: [], selected_id: "local-gpt", asr_selected_id: null, asr_error: null, ...overrides,
  };
}
