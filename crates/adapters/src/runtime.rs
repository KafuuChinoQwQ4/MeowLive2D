//! 有界 NVIDIA 显存采样；仅用于离线联合运行的实测，不由配置推测可用性。
use std::time::Duration;
#[derive(Clone, Debug)]
pub struct GpuSample {
    pub name: String,
    pub total_mib: u32,
    pub used_mib: u32,
}
pub async fn sample_gpu() -> Result<GpuSample, String> {
    let mut command = tokio::process::Command::new("nvidia-smi");
    command
        .args([
            "--id=0",
            "--query-gpu=name,memory.total,memory.used",
            "--format=csv,noheader,nounits",
        ])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(3), command.output())
        .await
        .map_err(|_| "GPU 采样超时")?
        .map_err(|_| "无法运行 nvidia-smi，离线资源尚未验证")?;
    if !output.status.success() || output.stdout.len() > 4096 {
        return Err("GPU 采样失败".into());
    }
    parse(&String::from_utf8_lossy(&output.stdout))
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
}
