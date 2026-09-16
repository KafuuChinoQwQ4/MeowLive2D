//! Linux / WSL 主服务进程入口。
#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let path = match args.as_slice() {
        [] => "config/server.example.toml",
        [flag] if flag == "--help" || flag == "-h" => {
            println!("meowlive-server [--config config/server.local.toml]");
            return std::process::ExitCode::SUCCESS;
        }
        [flag, path] if flag == "--config" => path,
        _ => {
            eprintln!("用法：meowlive-server [--config PATH]");
            return std::process::ExitCode::FAILURE;
        }
    };
    match meowlive_server::bootstrap::run(std::path::Path::new(path)).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
