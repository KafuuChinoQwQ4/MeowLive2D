//! Apply a verified WSL selection without replacing unrelated local configuration.
use meowlive_protocol::desktop_environment::EnvironmentSnapshot;
use toml_edit::{DocumentMut, value};

pub fn configure(original: &str, environment: &EnvironmentSnapshot) -> Result<String, String> {
    if environment.busy
        || !environment.backend.ready
        || !environment
            .models
            .iter()
            .any(|m| m.id == "gpt-sovits-v2" && m.selected && m.downloaded)
    {
        return Err("请先安装训练后端，下载并选用 GPT-SoVITS v2".into());
    }
    let distro = environment
        .selected_distro
        .as_deref()
        .filter(|s| !s.is_empty() && !s.chars().any(char::is_control))
        .ok_or("请选择可用 WSL2 发行版")?;
    let mut doc: DocumentMut = original
        .parse()
        .map_err(|_| "主服务配置无法解析，未进行修改")?;
    doc["training"]["enabled"] = value(environment.backend.gpu);
    doc["training"]["managed_inference"] = value(true);
    doc["training"]["wsl_distribution"] = value(distro);
    for (key, path) in [
        ("wsl_python", "/opt/meowlive-voice/venv/bin/python"),
        ("wsl_engine_root", "/opt/meowlive-voice/engine"),
        (
            "wsl_runner",
            "/opt/meowlive-voice/scripts/train-gpt-sovits.py",
        ),
        (
            "wsl_bridge",
            "/opt/meowlive-voice/scripts/wsl-training-bridge.py",
        ),
        (
            "default_gpt_weights",
            "/opt/meowlive-voice/models/gpt-sovits-v2/GPT_SoVITS/pretrained_models/gsv-v2final-pretrained/s1bert25hz-5kh-longer-epoch=12-step=369668.ckpt",
        ),
        (
            "default_sovits_weights",
            "/opt/meowlive-voice/models/gpt-sovits-v2/GPT_SoVITS/pretrained_models/gsv-v2final-pretrained/s2G2333k.pth",
        ),
    ] {
        doc["training"][key] = value(path);
    }
    let asr = environment
        .models
        .iter()
        .find(|m| m.selected && m.downloaded && m.capability == "transcription");
    if asr.is_some_and(|m| !m.id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')) {
        return Err("语音识别模型标识无效".into());
    }
    doc["training"]["asr_model"] = value(
        asr.map(|m| format!("/opt/meowlive-voice/models/{}", m.id))
            .unwrap_or_default(),
    );
    doc["speech"]["base_url"] = value("http://127.0.0.1:9880");
    // ResourceSynthesizer maps the actual Windows storage path into the selected distribution.
    doc["resources"]["engine_directory"] = value("");
    Ok(doc.to_string())
}
