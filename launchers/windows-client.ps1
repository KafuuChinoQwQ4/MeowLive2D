[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$SessionDirectory)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$session = [IO.Path]::GetFullPath($SessionDirectory)
$statusPath = Join-Path $session 'status.json'
function Write-Status([string]$State, [string]$Message, [int]$ChildId = 0) {
    $value = @{ state = $State; message = $Message; pid = $ChildId } | ConvertTo-Json -Compress
    [IO.File]::WriteAllText($statusPath + '.tmp', $value, [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath ($statusPath + '.tmp') -Destination $statusPath -Force
}
function Test-Lease {
    $lease = Join-Path $session 'lease'
    return (Test-Path -LiteralPath $lease) -and (([DateTime]::UtcNow - (Get-Item -LiteralPath $lease).LastWriteTimeUtc).TotalSeconds -lt 12)
}
$owner = $null
try {
    if (-not (Test-Lease) -or (Test-Path -LiteralPath (Join-Path $session 'stop'))) { Write-Status 'stopped' '启动已取消。'; exit 0 }
    $jobFile = Join-Path $session 'job.json'
    if ((Get-Item -LiteralPath $jobFile).Length -gt 32768) { throw '启动配置过大。' }
    $job = Get-Content -LiteralPath $jobFile -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($job.schema_version -ne 1) { throw '启动配置版本无效。' }
    $executable = [IO.Path]::GetFullPath([string]$job.executable)
    $configuration = [IO.Path]::GetFullPath([string]$job.configuration)
    if ([IO.Path]::GetFileName($executable) -cne 'meowlive-client.exe') { throw '只允许启动 MeowLive2D 执行端。' }
    if (-not (Test-Path -LiteralPath $executable -PathType Leaf) -or -not (Test-Path -LiteralPath $configuration -PathType Leaf)) { throw '执行端或配置文件不存在。' }
    if ($configuration -match '["\x00-\x1f]') { throw '配置路径包含无效字符。' }
    Write-Status 'starting' '正在启动 Windows 执行端。'
    Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Diagnostics;
using System.Runtime.InteropServices;
public sealed class MeowWindowsClientOwner : IDisposable {
    [StructLayout(LayoutKind.Sequential)] struct BasicLimit { public long PerProcessUserTimeLimit, PerJobUserTimeLimit; public uint LimitFlags; public UIntPtr MinimumWorkingSetSize, MaximumWorkingSetSize; public uint ActiveProcessLimit; public UIntPtr Affinity; public uint PriorityClass, SchedulingClass; }
    [StructLayout(LayoutKind.Sequential)] struct IoCounters { public ulong ReadOperationCount, WriteOperationCount, OtherOperationCount, ReadTransferCount, WriteTransferCount, OtherTransferCount; }
    [StructLayout(LayoutKind.Sequential)] struct ExtendedLimit { public BasicLimit BasicLimitInformation; public IoCounters IoInfo; public UIntPtr ProcessMemoryLimit, JobMemoryLimit, PeakProcessMemoryUsed, PeakJobMemoryUsed; }
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern IntPtr CreateJobObject(IntPtr attributes, string name);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool SetInformationJobObject(IntPtr job, int infoClass, IntPtr info, uint length);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
    IntPtr job; Process child; readonly object logLock = new object(); string log;
    public bool HasExited { get { return child == null || child.HasExited; } }
    public int Id { get { return child == null ? 0 : child.Id; } }
    void Append(string line) {
        if (line == null) return;
        try { lock(logLock) { if(File.Exists(log) && new FileInfo(log).Length > 2 * 1024 * 1024) File.WriteAllText(log, "[earlier output truncated]\n"); File.AppendAllText(log, line.Substring(0, Math.Min(line.Length, 4096)) + Environment.NewLine); } } catch {}
    }
    public MeowWindowsClientOwner(string executable, string configuration, string logPath) {
        log = logPath;
        job = CreateJobObject(IntPtr.Zero, null);
        if (job == IntPtr.Zero) throw new InvalidOperationException("Cannot create Windows process owner");
        try {
            ExtendedLimit limits = new ExtendedLimit(); limits.BasicLimitInformation.LimitFlags = 0x2000;
            int size = Marshal.SizeOf(typeof(ExtendedLimit)); IntPtr memory = Marshal.AllocHGlobal(size);
            try { Marshal.StructureToPtr(limits, memory, false); if(!SetInformationJobObject(job, 9, memory, (uint)size)) throw new InvalidOperationException("Cannot configure Windows process owner"); }
            finally { Marshal.FreeHGlobal(memory); }
            ProcessStartInfo start = new ProcessStartInfo(executable, "--config \"" + configuration + "\"");
            start.WorkingDirectory = Path.GetDirectoryName(executable); start.UseShellExecute = false; start.CreateNoWindow = true;
            start.RedirectStandardOutput = true; start.RedirectStandardError = true;
            child = new Process(); child.StartInfo = start;
            child.OutputDataReceived += (sender, e) => Append(e.Data); child.ErrorDataReceived += (sender, e) => Append(e.Data);
            if(!child.Start()) throw new InvalidOperationException("Cannot start Windows client");
            if(!AssignProcessToJobObject(job, child.Handle)) { child.Kill(); throw new InvalidOperationException("Cannot own Windows client process"); }
            child.BeginOutputReadLine(); child.BeginErrorReadLine();
        } catch { Dispose(); throw; }
    }
    public void Dispose() {
        if(job != IntPtr.Zero) { CloseHandle(job); job = IntPtr.Zero; }
        if(child != null) { try { if(!child.HasExited) { child.Kill(); child.WaitForExit(5000); } } catch {} child.Dispose(); child = null; }
    }
}
'@
    if (-not (Test-Lease) -or (Test-Path -LiteralPath (Join-Path $session 'stop'))) { Write-Status 'stopped' '启动已取消。'; exit 0 }
    $owner = [MeowWindowsClientOwner]::new($executable, $configuration, (Join-Path $session 'client.log'))
    Write-Status 'running' 'Windows 进程已启动，正在等待主服务确认连接。' $owner.Id
    while (-not $owner.HasExited) {
        if ((Test-Path -LiteralPath (Join-Path $session 'stop')) -or -not (Test-Lease)) { break }
        Start-Sleep -Milliseconds 400
    }
    if ($owner.HasExited -and -not (Test-Path -LiteralPath (Join-Path $session 'stop')) -and (Test-Lease)) {
        Write-Status 'failed' '执行端已退出，请检查 client.log 和默认扬声器。'
    } else { Write-Status 'stopped' 'Windows 执行端已断开。' }
} catch {
    try { Write-Status 'failed' '无法启动 Windows 执行端，请检查程序、配置文件、默认扬声器或 Windows 互操作。' } catch { }
    exit 1
} finally { if ($null -ne $owner) { $owner.Dispose() } }
