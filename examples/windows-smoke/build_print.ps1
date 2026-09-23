# Run after build.ps1 has produced the Windows release binaries. The preview
# must be cancelled by the operator; this script never submits a print job.
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$stage = Join-Path $repo 'dist\windows-print-stage'
$output = Join-Path $repo 'dist\NivaWindowsPrintSmoke.exe'
Set-Location -LiteralPath $repo

foreach ($binary in @('target\release\niva.exe', 'target\release\win_packager.exe')) {
    if (-not (Test-Path -LiteralPath (Join-Path $repo $binary))) { throw "Build the release binary first: $binary" }
}

$uuid = [Guid]::NewGuid().ToString()
$profileRoot = (Resolve-Path -LiteralPath $env:APPDATA).Path
$profilePath = Join-Path $profileRoot ("windows-print-smoke_" + $uuid.Substring(0, 8))
if (Test-Path -LiteralPath $profilePath) { throw "Disposable print profile already exists: $profilePath" }
New-Item -ItemType Directory -Path $stage -Force | Out-Null
foreach ($name in @('print.html', 'print.js')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $stage $name) -Force
}
$config = Join-Path $stage 'niva.json'
$template = [System.IO.File]::ReadAllText((Join-Path $PSScriptRoot 'niva-print.json'))
[System.IO.File]::WriteAllText($config, $template.Replace('__UUID__', $uuid), [System.Text.UTF8Encoding]::new($false))
& (Join-Path $repo 'target\release\win_packager.exe') `
    --exe (Join-Path $repo 'target\release\niva.exe') `
    --save-as $output `
    --resource-dir $stage `
    --config $config
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$testExitCode = 1
try {
    & python -B -u (Join-Path $PSScriptRoot 'run_print.py') $output
    $testExitCode = $LASTEXITCODE
} finally {
    if (Test-Path -LiteralPath $profilePath) {
        $profile = Get-Item -LiteralPath $profilePath -Force
        if ($profile.FullName -ne $profilePath -or -not $profile.PSIsContainer -or $profile.LinkType -or
            (Split-Path -Parent $profile.FullName) -ne $profileRoot) {
            throw "Unsafe disposable print profile cleanup target: $profilePath"
        }
        for ($attempt = 0; $attempt -lt 10 -and (Test-Path -LiteralPath $profilePath); $attempt++) {
            try { Remove-Item -LiteralPath $profilePath -Recurse -Force -ErrorAction Stop }
            catch { Start-Sleep -Milliseconds 300 }
        }
        if (Test-Path -LiteralPath $profilePath) { throw "Disposable print profile cleanup failed: $profilePath" }
    }
}
exit $testExitCode
