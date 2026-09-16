# Run on Windows: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows-bootstrap.test.ps1
# Only parses the entry and calls extracted pure/read helpers with a fake WSL executable.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$tokens = $null
$parseErrors = $null
$entry = Join-Path (Split-Path $PSScriptRoot -Parent) 'launchers/start-windows.ps1'
$ast = [System.Management.Automation.Language.Parser]::ParseFile($entry, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count) { throw ($parseErrors.Message -join "`n") }
foreach ($name in @('Read-WslDistributions', 'Assert-NativeArgument', 'Resolve-WslProjectPath')) {
    $definition = $ast.Find({ param($node) $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $name }, $false)
    if ($null -eq $definition) { throw "Missing function: $name" }
    . ([scriptblock]::Create($definition.Extent.Text))
}
function Assert-Equal($Actual, $Expected, $Message) {
    if ($Actual -ne $Expected) { throw "$Message (expected $Expected, got $Actual)" }
}
function mock-wsl {
    $global:LASTEXITCODE = $script:exitCode
    if ($script:throwWsl) { throw 'WSL unavailable' }
    if ($args -contains '--quiet') { return $script:names }
    return $script:rows
}
$script:exitCode = 0
$script:throwWsl = $false
$script:names = @('Ubuntu', 'archlinux', 'Name.[test]')
$script:rows = @('  NAME STATE VERSION', '* Ubuntu Running 2', '  archlinux Stopped 1', '  Name.[test] Stopped 2')
$found = @(Read-WslDistributions 'mock-wsl')
Assert-Equal $found.Count 3 'English WSL list'
Assert-Equal $found[0].IsDefault $true 'Default marker'
Assert-Equal $found[1].Version 1 'WSL1 remains distinguishable'
Assert-Equal $found[2].Name 'Name.[test]' 'Regex metacharacters in distro names'
$script:names = @("U`0b`0u`0n`0t`0u`0")
$script:rows = @('  NAME STATE VERSION', '* Ubuntu 已停止 2')
$found = @(Read-WslDistributions 'mock-wsl')
Assert-Equal $found.Count 1 'NUL cleanup / localized state'
Assert-Equal $found[0].Version 2 'Localized WSL2 row'
$script:exitCode = 1
Assert-Equal @(Read-WslDistributions 'mock-wsl').Count 0 'Failed WSL command'
$script:exitCode = 0
$script:throwWsl = $true
Assert-Equal @(Read-WslDistributions 'mock-wsl').Count 0 'Unavailable WSL subsystem'
Assert-NativeArgument '/home/example/My Project' 'path'
Assert-NativeArgument '/home/example/semicolon;$(literal)' 'path'
foreach ($badPath in @('/tmp/a"b', "/tmp/a`nb")) {
    $rejected = $false
    try { Assert-NativeArgument $badPath 'path' } catch { $rejected = $true }
    Assert-Equal $rejected $true 'Unsupported native argument rejected'
}
Assert-Equal (Resolve-WslProjectPath '~/code/MeowLive2D' '/home/alice') '/home/alice/code/MeowLive2D' 'Tilde uses selected Linux user home'
Assert-Equal (Resolve-WslProjectPath 'code/My Project' '/home/bob') '/home/bob/code/My Project' 'Relative project path uses selected Linux home'
Assert-Equal (Resolve-WslProjectPath '/srv/MeowLive2D' '/home/bob') '/srv/MeowLive2D' 'Absolute custom path is preserved'
Assert-Equal (Resolve-WslProjectPath '~/code/$(literal)' '/home/alice') '/home/alice/code/$(literal)' 'Path is never evaluated as shell code'
Write-Host 'Windows bootstrap: syntax, detection, argument and portable path assertions passed.'
