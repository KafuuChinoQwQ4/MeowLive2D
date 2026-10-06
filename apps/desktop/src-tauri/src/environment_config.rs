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
    if distro == "Linux" {
        return Err("本机 Linux 语音环境尚未接入训练执行配置，未写入 WSL 路径".into());
    }
    let mut doc: DocumentMut = original
        .parse()
        .map_err(|_| "主服务配置无法解析，未进行修改")?;
    doc["training"]["enabled"] = value(environment.backend.gpu);
    doc["training"]["managed_inference"] = value(true);
    doc["training"]["wsl_distribution"] = value(distro);
    let engine_root = environment.backend.engine_root.trim_end_matches('/');
    let model_root = &environment.backend.model_root;
    for path in [
        engine_root,
        environment.backend.python_path.as_str(),
        model_root.as_str(),
    ] {
        if !path.starts_with('/') || path.chars().any(char::is_control) {
            return Err("后端路径无效，请重新检测环境".into());
        }
    }
    for (key, path) in [
        ("wsl_python", environment.backend.python_path.clone()),
        ("wsl_engine_root", engine_root.to_string()),
        (
            "wsl_runner",
            "/opt/meowlive-voice/scripts/train-gpt-sovits.py".to_string(),
        ),
        (
            "wsl_bridge",
            "/opt/meowlive-voice/scripts/wsl-training-bridge.py".to_string(),
        ),
        (
            "default_gpt_weights",
            format!(
                "{model_root}/GPT_SoVITS/pretrained_models/gsv-v2final-pretrained/s1bert25hz-5kh-longer-epoch=12-step=369668.ckpt"
            ),
        ),
        (
            "default_sovits_weights",
            format!(
                "{model_root}/GPT_SoVITS/pretrained_models/gsv-v2final-pretrained/s2G2333k.pth"
            ),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_linux_selection_is_not_serialized_as_a_wsl_distro() {
        let mut environment: EnvironmentSnapshot = serde_json::from_value(serde_json::json!({
            "phase":"idle", "busy":false, "message":"", "logs":[], "distros":[],
            "selectedDistro":"Linux", "backend":{"ready":true,"engineRoot":"/opt/engine",
            "pythonPath":"/opt/python","modelRoot":"/opt/models","gpu":false,"detail":""},
            "models":[{"id":"gpt-sovits-v2","name":"v2","capability":"training_inference",
            "downloaded":true,"selected":true}], "progress":100,"inferenceRunning":false
        }))
        .unwrap();
        environment.backend.ready = true;
        assert!(
            configure("", &environment)
                .unwrap_err()
                .contains("尚未接入")
        );
    }
}
