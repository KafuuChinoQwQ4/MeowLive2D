use axum::Router;
use meowlive_adapters::speech::gpt_sovits::GptSovitsConfig;
use meowlive_application::ports::speech::SynthesisRequest;
use std::time::Duration;

pub async fn engine(router: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (url, task)
}

pub fn config(base_url: String) -> GptSovitsConfig {
    GptSovitsConfig {
        base_url,
        reference_audio: "/engine/reference.wav".into(),
        prompt_text: "参考文本".into(),
        prompt_language: "zh".into(),
        text_language: "zh".into(),
        timeout: Duration::from_secs(2),
        max_audio_bytes: 1024,
    }
}

pub fn request() -> SynthesisRequest {
    SynthesisRequest {
        text: "你好".into(),
        voice_id: "default".into(),
    }
}
