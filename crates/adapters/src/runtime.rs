//! 有界 NVIDIA 显存采样；仅用于离线联合运行的实测，不由配置推测可用性。
use std::time::Duration;
#[derive(Clone, Debug)]
pub struct GpuSample {
    pub name: String,
    pub total_mib: u32,
    pub used_mib: u32,
}
pub async fn sample_gpu() -> Result<GpuSample, String> {
    sample_gpu_index(0).await
}
pub async fn sample_gpu_index(index: u16) -> Result<GpuSample, String> {
    if index > 15 {
        return Err("GPU 序号超出范围".into());
    }
    sample_gpu_command(tokio::process::Command::new(nvidia_smi_program()), index).await
}
fn nvidia_smi_program() -> std::path::PathBuf {
    resolve_nvidia_smi(
        std::env::var_os("PATH").as_deref(),
        std::path::Path::new("/usr/lib/wsl/lib/nvidia-smi"),
    )
}
fn resolve_nvidia_smi(
    search_path: Option<&std::ffi::OsStr>,
    wsl_program: &std::path::Path,
) -> std::path::PathBuf {
    if cfg!(target_os = "linux") {
        if let Some(program) = search_path
            .into_iter()
            .flat_map(std::env::split_paths)
            .map(|directory| directory.join("nvidia-smi"))
            .find(|program| program.is_file())
        {
            return program;
        }
        if wsl_program.is_file() {
            return wsl_program.into();
        }
    }
    "nvidia-smi".into()
}
async fn sample_gpu_command(
    mut command: tokio::process::Command,
    index: u16,
) -> Result<GpuSample, String> {
    let program = command
        .as_std()
        .get_program()
        .to_string_lossy()
        .into_owned();
    let context = format!("GPU {index} 采样（{program}）");
    command
        .args([
            &format!("--id={index}"),
            "--query-gpu=name,memory.total,memory.used",
            "--format=csv,noheader,nounits",
        ])
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(3), command.output())
        .await
        .map_err(|_| format!("{context}超时（3 秒），离线资源尚未验证"))?
        .map_err(|error| format!("{context}无法启动：{error}；离线资源尚未验证"))?;
    if !output.status.success() {
        let detail = diagnostic(&output.stderr, &output.stdout);
        return Err(format!("{context}失败（{}）：{detail}", output.status));
    }
    if output.stdout.len() > 4096 {
        return Err(format!("{context}输出超过 4096 字节，无法验证显存"));
    }
    parse(&String::from_utf8_lossy(&output.stdout)).map_err(|error| {
        format!(
            "{context}失败：{error}；{}",
            diagnostic(&output.stderr, &output.stdout)
        )
    })
}
fn diagnostic(stderr: &[u8], stdout: &[u8]) -> String {
    let detail = [stderr, stdout]
        .into_iter()
        .map(String::from_utf8_lossy)
        .map(|value| {
            value
                .chars()
                .filter(|c| !c.is_control() || c.is_whitespace())
                .collect::<String>()
        })
        .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("；");
    if detail.is_empty() {
        "命令未提供错误详情".into()
    } else {
        let mut shortened: String = detail.chars().take(1024).collect();
        if detail.chars().count() > 1024 {
            shortened.push('…');
        }
        shortened
    }
}
fn parse(value: &str) -> Result<GpuSample, String> {
    let fields: Vec<_> = value.trim().split(',').map(str::trim).collect();
    if fields.len() != 3 || fields[0].is_empty() || fields[0].len() > 128 {
        return Err("GPU 采样格式无效".into());
    }
    let total_mib = fields[1].parse::<u32>().map_err(|_| "显存总量无效")?;
    let used_mib = fields[2].parse::<u32>().map_err(|_| "显存占用无效")?;
    if total_mib == 0 || used_mib > total_mib {
        return Err("显存采样范围无效".into());
    }
    Ok(GpuSample {
        name: fields[0].into(),
        total_mib,
        used_mib,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn memory_is_measured_and_invalid_outputs_are_rejected() {
        let gpu = parse("NVIDIA RTX 4050 Laptop GPU, 6141, 3240\n").unwrap();
        assert_eq!(gpu.used_mib, 3240);
        for value in [
            "NVIDIA, N/A, N/A",
            "NVIDIA, 6141, 9999",
            "a, 0, 0",
            "a, 100, 10\nb, 100, 10",
        ] {
            assert!(parse(value).is_err());
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn selected_gpu_index_is_passed_to_the_sampler_command() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!("gpu-sample-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let script = root.join("nvidia-smi");
        let arguments = root.join("arguments");
        std::fs::write(
            &script,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$GPU_SAMPLE_ARGUMENTS\"\nprintf 'Test GPU, 8192, 256\\n'\n",
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut command = tokio::process::Command::new(&script);
        command.env("GPU_SAMPLE_ARGUMENTS", &arguments);
        let sample = sample_gpu_command(command, 3).await.unwrap();
        assert_eq!(sample.used_mib, 256);
        assert_eq!(
            std::fs::read_to_string(arguments).unwrap().lines().next(),
            Some("--id=3")
        );
        assert!(sample_gpu_index(16).await.is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn command_failure_retains_gpu_exit_status_and_driver_diagnostic() {
        let mut command = tokio::process::Command::new("/bin/sh");
        command.args([
            "-c",
            "printf 'Failed to initialize NVML: Driver/library version mismatch\\n' >&2; exit 9",
        ]);
        let error = sample_gpu_command(command, 3).await.unwrap_err();
        assert!(error.contains("GPU 3"), "{error}");
        assert!(error.contains("9"), "{error}");
        assert!(error.contains("Driver/library version mismatch"), "{error}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn command_failure_keeps_diagnostics_printed_on_stdout() {
        let mut command = tokio::process::Command::new("/bin/sh");
        command.args(["-c", "printf 'No devices were found\\n'; exit 6"]);
        let error = sample_gpu_command(command, 0).await.unwrap_err();
        assert!(error.contains("No devices were found"), "{error}");
    }

    #[tokio::test]
    async fn missing_sampler_reports_executable_and_operating_system_error() {
        let program = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!("missing-nvidia-smi-{}", uuid::Uuid::new_v4()));
        let error = sample_gpu_command(tokio::process::Command::new(&program), 2)
            .await
            .unwrap_err();
        assert!(error.contains("GPU 2"), "{error}");
        assert!(
            error.contains(program.file_name().unwrap().to_str().unwrap()),
            "{error}"
        );
        assert!(error.contains("os error"), "{error}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn wsl_sampler_is_available_without_its_directory_on_path_and_path_keeps_priority() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!("gpu-resolve-{}", uuid::Uuid::new_v4()));
        let bin = root.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let fallback = root.join("wsl-nvidia-smi");
        std::fs::write(&fallback, "fixture").unwrap();
        let selected = resolve_nvidia_smi(Some(bin.as_os_str()), &fallback);
        assert_eq!(selected, fallback);
        let primary = bin.join("nvidia-smi");
        std::fs::write(&primary, "fixture").unwrap();
        assert_eq!(
            resolve_nvidia_smi(Some(bin.as_os_str()), &fallback),
            primary
        );
        std::fs::remove_file(&primary).unwrap();
        std::fs::remove_file(&fallback).unwrap();
        assert_eq!(
            resolve_nvidia_smi(Some(bin.as_os_str()), &fallback),
            std::path::PathBuf::from("nvidia-smi")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
