//! 无设备依赖的模型包校验与显式目录安装入口。
use meowlive_desktop_runtime::assets::{
    ModelImportLimits, install_model_package, validate_model_package,
};
use std::path::Path;

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
fn run() -> Result<(), String> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.len() == 1 && matches!(arguments[0].as_str(), "--help" | "-h") {
        println!(
            "Usage: meowlive-model validate MODEL_DIRECTORY\n       meowlive-model install MODEL_DIRECTORY Live2DModels INSTALL_NAME\nInstall never overwrites an existing model. Restart VTS to refresh newly installed models."
        );
        return Ok(());
    }
    if !matches!(arguments.as_slice(),[command,_] if command=="validate")
        && !matches!(arguments.as_slice(),[command,_,_,_] if command=="install")
    {
        return Err(
            "请使用 validate MODEL_DIRECTORY 或 install MODEL_DIRECTORY Live2DModels INSTALL_NAME"
                .into(),
        );
    }
    let package = validate_model_package(Path::new(&arguments[1]), &ModelImportLimits::default())
        .map_err(|e| format!("模型校验失败：{e}"))?;
    println!(
        "模型：{}\n模型根目录：{}\n文件数：{}\n总字节：{}",
        package.model_file().display(),
        package.root().display(),
        package.file_count(),
        package.total_bytes()
    );
    if arguments[0] == "install" {
        let installed = install_model_package(&package, Path::new(&arguments[2]), &arguments[3])
            .map_err(|e| format!("模型安装失败：{e}"))?;
        println!(
            "安装完成：{}\n请重启 VTube Studio 后刷新模型列表。",
            installed.installed_path.display()
        );
    }
    Ok(())
}
