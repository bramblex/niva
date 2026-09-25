param([switch]$Interactive)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$stage = Join-Path $repo 'dist\windows-smoke-stage'
$cspStage = Join-Path $repo 'dist\windows-csp-stage'
$apiStage = Join-Path $repo 'dist\windows-api-stage'
$streamStage = Join-Path $repo 'dist\windows-stream-stage'
$output = Join-Path $repo 'dist\NivaWindowsSmoke.exe'
Set-Location -LiteralPath $repo

& npm run build --workspace=packages/devtools
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& cargo build --release -p win_packager
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& cargo build --release -p niva
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

New-Item -ItemType Directory -Path $stage -Force | Out-Null
foreach ($name in @('niva.json', 'niva-csp.json', 'index.html', 'iframe.html', 'cross.html', 'child.html', 'csp.html', 'csp.js', 'fixture-protocol.js', 'probe.txt')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $stage $name) -Force
}

New-Item -ItemType Directory -Path $cspStage -Force | Out-Null
foreach ($name in @('csp.html', 'csp.js', 'fixture-protocol.js', 'probe.txt')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $cspStage $name) -Force
}

& (Join-Path $repo 'target\release\win_packager.exe') `
    --exe (Join-Path $repo 'target\release\niva.exe') `
    --save-as (Join-Path $repo 'dist\NivaWindowsCspSmoke.exe') `
    --resource-dir $cspStage `
    --config (Join-Path $PSScriptRoot 'niva-csp.json')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& python -B -u (Join-Path $PSScriptRoot 'run_csp.py') (Join-Path $repo 'dist\NivaWindowsCspSmoke.exe')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

New-Item -ItemType Directory -Path $streamStage -Force | Out-Null
foreach ($name in @('stream.html', 'stream.js', 'fixture-protocol.js')) {
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
foreach ($name in @('api-safe.html', 'api-safe.js', 'api-child.html', 'fixture-protocol.js', 'probe.txt')) {
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
