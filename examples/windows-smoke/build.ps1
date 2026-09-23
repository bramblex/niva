param([switch]$Interactive)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$stage = Join-Path $repo 'dist\windows-smoke-stage'
$cspStage = Join-Path $repo 'dist\windows-csp-stage'
$apiStage = Join-Path $repo 'dist\windows-api-stage'
$streamStage = Join-Path $repo 'dist\windows-stream-stage'
$clearStage = Join-Path $repo 'dist\windows-clear-stage'
$clipboardStage = Join-Path $repo 'dist\windows-clipboard-stage'
$output = Join-Path $repo 'dist\NivaWindowsSmoke.exe'
Set-Location -LiteralPath $repo

& npm run build --workspace=packages/devtools
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& cargo build --release -p win_packager
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& cargo build --release -p niva
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

New-Item -ItemType Directory -Path $stage -Force | Out-Null
foreach ($name in @('niva.json', 'niva-csp.json', 'index.html', 'iframe.html', 'cross.html', 'child.html', 'csp.html', 'csp.js', 'probe.txt')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $stage $name) -Force
}
Copy-Item -LiteralPath (Join-Path $repo 'packages\devtools\build\__niva_compat') -Destination $stage -Recurse -Force

New-Item -ItemType Directory -Path $cspStage -Force | Out-Null
foreach ($name in @('csp.html', 'csp.js', 'probe.txt')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $cspStage $name) -Force
}
Copy-Item -LiteralPath (Join-Path $repo 'packages\devtools\build\__niva_compat') -Destination $cspStage -Recurse -Force

& (Join-Path $repo 'target\release\win_packager.exe') `
    --exe (Join-Path $repo 'target\release\niva.exe') `
    --save-as (Join-Path $repo 'dist\NivaWindowsCspSmoke.exe') `
    --resource-dir $cspStage `
    --config (Join-Path $PSScriptRoot 'niva-csp.json')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& python -B -u (Join-Path $PSScriptRoot 'run_csp.py') (Join-Path $repo 'dist\NivaWindowsCspSmoke.exe')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$clearUuid = [Guid]::NewGuid().ToString()
$profileRoot = (Resolve-Path -LiteralPath $env:APPDATA).Path
$profilePath = Join-Path $profileRoot ("windows-clear-smoke_" + $clearUuid.Substring(0, 8))
if (Test-Path -LiteralPath $profilePath) { throw "Disposable profile already exists: $profilePath" }
New-Item -ItemType Directory -Path $clearStage -Force | Out-Null
foreach ($name in @('clear.html', 'clear.js')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $clearStage $name) -Force
}
$clearConfig = Join-Path $clearStage 'niva.json'
$clearTemplate = [System.IO.File]::ReadAllText((Join-Path $PSScriptRoot 'niva-clear.json'))
[System.IO.File]::WriteAllText($clearConfig, $clearTemplate.Replace('__UUID__', $clearUuid), [System.Text.UTF8Encoding]::new($false))
& (Join-Path $repo 'target\release\win_packager.exe') `
    --exe (Join-Path $repo 'target\release\niva.exe') `
    --save-as (Join-Path $repo 'dist\NivaWindowsClearSmoke.exe') `
    --resource-dir $clearStage `
    --config $clearConfig
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$clearExitCode = 1
try {
    & python -B -u (Join-Path $PSScriptRoot 'run_clear.py') (Join-Path $repo 'dist\NivaWindowsClearSmoke.exe')
    $clearExitCode = $LASTEXITCODE
} finally {
    if (Test-Path -LiteralPath $profilePath) {
        $profile = Get-Item -LiteralPath $profilePath -Force
        if ($profile.FullName -ne $profilePath -or -not $profile.PSIsContainer -or $profile.LinkType -or
            (Split-Path -Parent $profile.FullName) -ne $profileRoot) {
            throw "Unsafe disposable profile cleanup target: $profilePath"
        }
        for ($attempt = 0; $attempt -lt 10 -and (Test-Path -LiteralPath $profilePath); $attempt++) {
            try { Remove-Item -LiteralPath $profilePath -Recurse -Force -ErrorAction Stop }
            catch { Start-Sleep -Milliseconds 300 }
        }
        if (Test-Path -LiteralPath $profilePath) { throw "Disposable profile cleanup failed: $profilePath" }
    }
}
if ($clearExitCode -ne 0) { exit $clearExitCode }

$clipboardUuid = [Guid]::NewGuid().ToString()
$clipboardProfilePath = Join-Path $profileRoot ("windows-clipboard-smoke_" + $clipboardUuid.Substring(0, 8))
if (Test-Path -LiteralPath $clipboardProfilePath) { throw "Disposable profile already exists: $clipboardProfilePath" }
New-Item -ItemType Directory -Path $clipboardStage -Force | Out-Null
foreach ($name in @('clipboard.html', 'clipboard.js')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $clipboardStage $name) -Force
}
$clipboardConfig = Join-Path $clipboardStage 'niva.json'
$clipboardTemplate = [System.IO.File]::ReadAllText((Join-Path $PSScriptRoot 'niva-clipboard.json'))
[System.IO.File]::WriteAllText($clipboardConfig, $clipboardTemplate.Replace('__UUID__', $clipboardUuid), [System.Text.UTF8Encoding]::new($false))
& (Join-Path $repo 'target\release\win_packager.exe') `
    --exe (Join-Path $repo 'target\release\niva.exe') `
    --save-as (Join-Path $repo 'dist\NivaWindowsClipboardSmoke.exe') `
    --resource-dir $clipboardStage `
    --config $clipboardConfig
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$clipboardExitCode = 1
try {
    & python -B -u (Join-Path $PSScriptRoot 'run_clipboard.py') (Join-Path $repo 'dist\NivaWindowsClipboardSmoke.exe')
    $clipboardExitCode = $LASTEXITCODE
} finally {
    if (Test-Path -LiteralPath $clipboardProfilePath) {
        $profile = Get-Item -LiteralPath $clipboardProfilePath -Force
        if ($profile.FullName -ne $clipboardProfilePath -or -not $profile.PSIsContainer -or $profile.LinkType -or
            (Split-Path -Parent $profile.FullName) -ne $profileRoot) {
            throw "Unsafe disposable profile cleanup target: $clipboardProfilePath"
        }
        for ($attempt = 0; $attempt -lt 10 -and (Test-Path -LiteralPath $clipboardProfilePath); $attempt++) {
            try { Remove-Item -LiteralPath $clipboardProfilePath -Recurse -Force -ErrorAction Stop }
            catch { Start-Sleep -Milliseconds 300 }
        }
        if (Test-Path -LiteralPath $clipboardProfilePath) { throw "Disposable profile cleanup failed: $clipboardProfilePath" }
    }
}
if ($clipboardExitCode -ne 0) { exit $clipboardExitCode }

New-Item -ItemType Directory -Path $streamStage -Force | Out-Null
foreach ($name in @('stream.html', 'stream.js')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $streamStage $name) -Force
}
[System.IO.File]::WriteAllText((Join-Path $streamStage 'large.txt'), ('R' * 150000), [System.Text.UTF8Encoding]::new($false))
& (Join-Path $repo 'target\release\win_packager.exe') `
    --exe (Join-Path $repo 'target\release\niva.exe') `
    --save-as (Join-Path $repo 'dist\NivaWindowsStreamSmoke.exe') `
    --resource-dir $streamStage `
    --config (Join-Path $PSScriptRoot 'niva-stream.json')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& python -B -u (Join-Path $PSScriptRoot 'run_stream.py') (Join-Path $repo 'dist\NivaWindowsStreamSmoke.exe')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

& (Join-Path $repo 'target\release\win_packager.exe') `
    --exe (Join-Path $repo 'target\release\niva.exe') `
    --save-as $output `
    --resource-dir $stage `
    --config (Join-Path $stage 'niva.json')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$runArgs = @('-B', '-u', (Join-Path $PSScriptRoot 'run.py'), $output)
if ($Interactive) { $runArgs += '--ui' }
& python @runArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

New-Item -ItemType Directory -Path $apiStage -Force | Out-Null
foreach ($name in @('api-safe.html', 'api-safe.js', 'api-child.html', 'nav-two.html', 'nav-child.js', 'probe.txt')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $apiStage $name) -Force
}
Copy-Item -LiteralPath (Join-Path $repo 'packages\devtools\public\icon.png') -Destination (Join-Path $apiStage 'icon.png') -Force
& (Join-Path $repo 'target\release\win_packager.exe') `
    --exe (Join-Path $repo 'target\release\niva.exe') `
    --save-as (Join-Path $repo 'dist\NivaWindowsApiSmoke.exe') `
    --resource-dir $apiStage `
    --config (Join-Path $PSScriptRoot 'niva-api.json')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& python -B -u (Join-Path $PSScriptRoot 'run_api.py') (Join-Path $repo 'dist\NivaWindowsApiSmoke.exe')
exit $LASTEXITCODE
