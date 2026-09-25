param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('check', 'install')]
    [string]$Action,
    [switch]$Elevated
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$WarningPreference = 'SilentlyContinue'
$RuntimeClientId = '{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
$BootstrapperUrl = 'https://go.microsoft.com/fwlink/p/?LinkId=2124703'

$RuntimeRegistryPath = "SOFTWARE\Microsoft\EdgeUpdate\Clients\$RuntimeClientId"

function Get-RuntimeVersion([Microsoft.Win32.RegistryHive]$Hive) {
    $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        $Hive,
        [Microsoft.Win32.RegistryView]::Registry32
    )
    try {
        $key = $base.OpenSubKey($RuntimeRegistryPath)
        if (!$key) {
            return $null
        }
        try {
            $value = $key.GetValue('pv', $null)
        } finally {
            $key.Close()
        }
        if ($value -and $value -ne '0.0.0.0') {
            $parsed = [version]::Parse([string]$value)
            return $parsed.ToString(4)
        }
    } catch {
        # Missing or malformed registration means this hive has no usable runtime.
    } finally {
        $base.Close()
    }
    return $null
}

function Get-RuntimeState {
    # Microsoft's distribution documentation specifies the 32-bit registry
    # view for the machine registration on 64-bit Windows (WOW6432Node) and
    # the regular SOFTWARE path on 32-bit Windows. Registry32 resolves that
    # documented view without depending on this PowerShell process bitness.
    $machineVersion = Get-RuntimeVersion ([Microsoft.Win32.RegistryHive]::LocalMachine)
    $userVersion = Get-RuntimeVersion ([Microsoft.Win32.RegistryHive]::CurrentUser)
    return @{ machineVersion = $machineVersion; userVersion = $userVersion }
}

if ($Action -eq 'check') {
    [Console]::Out.WriteLine((Get-RuntimeState | ConvertTo-Json -Compress))
    return
}

$installerPath = Join-Path ([IO.Path]::GetTempPath()) ("niva-webview2-" + [Guid]::NewGuid().ToString('N') + '.exe')
try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -Uri $BootstrapperUrl -OutFile $installerPath -UseBasicParsing

    $signature = Get-AuthenticodeSignature -FilePath $installerPath
    if ($signature.Status.ToString() -ne 'Valid' -or
        !$signature.SignerCertificate -or
        $signature.SignerCertificate.Subject -notmatch '(?i)O=Microsoft Corporation') {
        throw 'Downloaded WebView2 bootstrapper does not have a valid Microsoft Corporation signature.'
    }

    $start = @{
        FilePath = $installerPath
        ArgumentList = @('/silent', '/install')
        Wait = $true
        PassThru = $true
        WindowStyle = 'Hidden'
    }
    if ($Elevated) {
        $start.Verb = 'RunAs'
    }
    $process = Start-Process @start
    $state = Get-RuntimeState
    $installStatus = if ($process.ExitCode -eq 0) { 'complete' } else { 'failed' }
    $result = @{
        status = $installStatus
        exitCode = $process.ExitCode
        machineVersion = $state.machineVersion
        userVersion = $state.userVersion
    }
    [Console]::Out.WriteLine(($result | ConvertTo-Json -Compress))
} catch {
    $nativeError = $_.Exception.NativeErrorCode
    $status = if ($nativeError -eq 1223) { 'cancelled' } else { 'failed' }
    [Console]::Out.WriteLine((@{ status = $status; error = $_.Exception.Message } | ConvertTo-Json -Compress))
} finally {
    Remove-Item -LiteralPath $installerPath -Force -ErrorAction SilentlyContinue
}
