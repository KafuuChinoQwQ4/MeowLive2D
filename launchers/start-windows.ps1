[CmdletBinding()]
param(
    [string]$Distribution,
    [string]$ProjectPath,
    [switch]$CheckOnly,
    [switch]$NoOpen
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Show-InstallHelp {
    Write-Host ''
    Write-Host '请先安装 WSL2，完成后重新打开本启动器。' -ForegroundColor Yellow
    Write-Host '1. 右键开始菜单，打开“终端（管理员）”，执行：'
    Write-Host '   wsl --install -d Ubuntu' -ForegroundColor Cyan
    Write-Host '2. 按提示重启 Windows；打开 Ubuntu，完成 Linux 用户名和密码设置。'
    Write-Host '3. 执行 wsl --list --verbose，确认 Ubuntu 的 VERSION 为 2。'
    Write-Host '   如为 1，先备份发行版中的重要资料，再执行：wsl --set-version Ubuntu 2'
    Write-Host '4. 将项目和开发依赖准备在 Linux 中，再次运行本启动器。'
    Write-Host '微软安装说明：https://learn.microsoft.com/zh-cn/windows/wsl/install'
    Write-Host '安装未成功时按官方说明检查 Windows 版本、虚拟化和重启要求。'
}

function Read-WslDistributions {
    param([string]$Executable)
    # Names come from --quiet, so localized column headings/state text need not be interpreted.
    try {
        $namesOutput = @(& $Executable --list --quiet 2>&1)
        $namesExit = $LASTEXITCODE
        $verboseOutput = @(& $Executable --list --verbose 2>&1)
        $verboseExit = $LASTEXITCODE
    } catch { return @() }
    if ($namesExit -ne 0 -or $verboseExit -ne 0) { return @() }
    $names = @(($namesOutput -join "`n").Replace([string][char]0, '').Split("`n") |
        ForEach-Object { $_.Trim() } | Where-Object { $_ })
    $rows = (($verboseOutput -join "`n").Replace([string][char]0, '')).Split("`n")
    foreach ($name in $names) {
        $pattern = '^\s*(?<default>\*)?\s*' + [regex]::Escape($name) + '\s+.+?\s+(?<version>[12])\s*$'
        foreach ($row in $rows) {
            if ($row -match $pattern) {
                [pscustomobject]@{
                    Name = $name
                    Version = [int]$Matches.version
                    IsDefault = $Matches['default'] -eq '*'
                }
                break
            }
        }
    }
}

function Test-ControlPanel {
    try {
        $status = Invoke-RestMethod -Uri 'http://127.0.0.1:1420/api/launcher/status' -TimeoutSec 2
        return ($status.schema_version -eq 1 -and $status.session_token -match '^[0-9a-f]{64}$')
    } catch { return $false }
}

function Assert-NativeArgument {
    param([string]$Value, [string]$Label)
    # Windows PowerShell 5.1's native quoting cannot reliably preserve embedded double quotes.
    if ($Value -match '[\x00-\x1f"]') {
        throw "$Label 不能包含双引号或控制字符；请使用普通目录名。"
    }
}

function Resolve-WslProjectPath {
    param([string]$Value, [string]$LinuxHome)
    if ($Value.StartsWith('/')) { return $Value }
    if (-not $LinuxHome.StartsWith('/')) { throw '无法确定当前 Linux 用户的主目录，请填写 Linux 项目绝对路径。' }
    if ($Value -eq '~') { return $LinuxHome }
    if ($Value.StartsWith('~/')) { return $LinuxHome.TrimEnd('/') + '/' + $Value.Substring(2) }
    return $LinuxHome.TrimEnd('/') + '/' + $Value
}

try {
    Write-Host 'MeowLive2D · Windows 启动检查' -ForegroundColor Magenta
    $wslCommand = Get-Command wsl.exe -ErrorAction SilentlyContinue
    if ($null -eq $wslCommand) { Show-InstallHelp; exit 2 }
    $wsl = $wslCommand.Source
    $distributions = @(Read-WslDistributions -Executable $wsl)
    $available = @($distributions | Where-Object { $_.Version -eq 2 -and $_.Name -notmatch '^docker-desktop(?:-data)?$' })
    if ($available.Count -eq 0) {
        if ($distributions.Count -gt 0) {
            Write-Host '没有可供本项目使用的 WSL2 Linux 发行版。当前检测结果：'
            $distributions | Format-Table Name, Version | Out-Host
            Write-Host '已有 WSL1 时，可备份后运行：wsl --set-version "发行版名称" 2'
        }
        Show-InstallHelp
        exit 2
    }
    Write-Host '检测到以下 WSL2 发行版：' -ForegroundColor Green
    $available | Format-Table Name, Version, IsDefault | Out-Host
    if ($CheckOnly) {
        if ($Distribution -and -not ($available.Name -contains $Distribution)) {
            throw "指定的发行版 $Distribution 不是可用的 WSL2 发行版。"
        }
        Write-Host 'WSL2 检查通过；未启动服务、安装软件或修改系统。'
        exit 0
    }

    $uncDistribution = $null
    $uncProjectPath = $null
    if ($PSScriptRoot -match '^\\\\(?:wsl\.localhost|wsl\$)\\([^\\]+)(\\.*)?$') {
        $uncDistribution = $Matches[1]
        $uncProjectPath = (($Matches[2] -replace '\\', '/') -replace '/launchers$', '')
    }
    if (-not $Distribution -and $uncDistribution) { $Distribution = $uncDistribution }
    if (-not $Distribution) {
        if ($available.Count -eq 1) { $Distribution = $available[0].Name }
        else {
            Write-Host '请选择保存 MeowLive2D 项目的发行版：'
            for ($i = 0; $i -lt $available.Count; $i++) { Write-Host ('  {0}. {1}' -f ($i + 1), $available[$i].Name) }
            $selection = Read-Host '输入编号'
            $choice = 0
            if (-not [int]::TryParse($selection, [ref]$choice) -or $choice -lt 1 -or $choice -gt $available.Count) {
                throw '编号无效，请重新打开启动器并选择列表中的编号。'
            }
            $Distribution = $available[$choice - 1].Name
        }
    }
    if (-not ($available.Name -contains $Distribution)) {
        throw "发行版 $Distribution 尚未使用 WSL2。请运行 wsl --list --verbose 检查，按上方安装说明准备后重试。"
    }
    Assert-NativeArgument $Distribution '发行版名称'
    if (-not $ProjectPath -and $Distribution -eq $uncDistribution) { $ProjectPath = $uncProjectPath }
    if (-not $ProjectPath -and (Test-Path -LiteralPath (Join-Path (Split-Path $PSScriptRoot -Parent) 'launchers/start.sh'))) {
        $translated = @(& $wsl --distribution $Distribution --exec wslpath -a -u (Split-Path $PSScriptRoot -Parent) 2>&1)
        if ($LASTEXITCODE -eq 0) { $ProjectPath = ($translated -join "`n").Trim() }
    }
    if (-not $ProjectPath) {
        Write-Host '输入该发行版内的项目路径，例如 ~/code/MeowLive2D；相对路径以当前 Linux 用户的主目录为起点。'
        $ProjectPath = Read-Host 'Linux 项目路径'
    }
    Assert-NativeArgument $ProjectPath '项目路径'
    if (-not $ProjectPath.StartsWith('/')) {
        $linuxHomeOutput = @(& $wsl --distribution $Distribution --exec printenv HOME 2>&1)
        if ($LASTEXITCODE -ne 0) { throw '无法读取当前 Linux 用户的主目录，请用 -ProjectPath 指定完整项目路径。' }
        $ProjectPath = Resolve-WslProjectPath $ProjectPath ($linuxHomeOutput -join "`n").Trim()
    }
    Assert-NativeArgument $ProjectPath '项目路径'
    & $wsl --distribution $Distribution --exec test -f ($ProjectPath.TrimEnd('/') + '/launchers/start.sh')
    if ($LASTEXITCODE -ne 0) {
        throw "在 $Distribution 指定的项目目录中没有找到 ./launchers/start.sh。请先准备完整项目，或用 -ProjectPath 指定正确路径。"
    }

    Write-Host "项目：$Distribution : ./ （当前项目根目录）"
    Write-Host '控制面板：http://127.0.0.1:1420' -ForegroundColor Cyan
    if (Test-ControlPanel) {
        Write-Host '检测到已有控制面板。若不是当前项目，请先关闭原启动终端再运行。'
        if (-not $NoOpen) { Start-Process 'http://127.0.0.1:1420' }
        exit 0
    }
    Write-Host '请保持此窗口打开。先检查环境和语音模型；主服务会自动启动，就绪后自动开启 TTS 和 Windows 执行端。'
    Write-Host '首次安装前端依赖可能稍久；未自动打开时，手动访问上方网址。'
    $browserJob = $null
    try {
        if (-not $NoOpen) {
            $browserJob = Start-Job -ScriptBlock {
                for ($attempt = 0; $attempt -lt 600; $attempt++) {
                    try {
                        $status = Invoke-RestMethod 'http://127.0.0.1:1420/api/launcher/status' -TimeoutSec 2
                        if ($status.schema_version -eq 1 -and $status.session_token -match '^[0-9a-f]{64}$') {
                            Start-Process 'http://127.0.0.1:1420'
                            return
                        }
                    } catch { }
                    Start-Sleep -Seconds 1
                }
            }
        }
        # Arguments are passed directly to wsl.exe; Linux paths are never interpolated into shell code.
        & $wsl --distribution $Distribution --cd $ProjectPath --exec bash --login ./launchers/start.sh --no-open
        $launchExit = $LASTEXITCODE
    } finally {
        if ($null -ne $browserJob) { Stop-Job $browserJob; Remove-Job $browserJob -Force }
    }
    if ($launchExit -ne 0) { Write-Host '启动未完成。请按上方错误处理；Linux 需有 Node.js 22.12+、npm、Python 3.11+，主服务另需 Rust。' -ForegroundColor Yellow }
    exit $launchExit
} catch {
    Write-Host $_.Exception.Message -ForegroundColor Red
    Write-Host '帮助：./launchers/README.md；微软 WSL：https://learn.microsoft.com/zh-cn/windows/wsl/install'
    exit 1
}
