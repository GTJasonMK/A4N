param(
    [Parameter(Mandatory = $true)]
    [string]$Archive
)

$ErrorActionPreference = "Stop"

foreach ($name in "A4N_DATABASE_URL", "A4N_QDRANT_URL", "A4N_JWT_SECRET") {
    if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name, "Process"))) {
        throw "$name is required for release smoke"
    }
}
if (-not (Test-Path -LiteralPath $Archive -PathType Leaf)) {
    throw "release archive does not exist: $Archive"
}

$Archive = (Resolve-Path -LiteralPath $Archive).Path
$ProjectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$ManifestTool = Join-Path $ProjectRoot "scripts\release_manifest.rb"
$ServiceRows = @(& ruby $ManifestTool service-rows)
if ($LASTEXITCODE -ne 0) { throw "unable to read sidecar service catalog" }
$Services = @($ServiceRows | ForEach-Object {
    $fields = $_ -split "`t", 5
    [pscustomobject]@{
        name = $fields[0]
        binary = $fields[1]
        addr_env = $fields[2]
        url_env = $fields[3]
        health_path = $fields[4]
    }
})
$ToolRows = @(& ruby $ManifestTool tool-rows)
if ($LASTEXITCODE -ne 0) { throw "unable to read release tool catalog" }
$ReleaseTools = @($ToolRows | ForEach-Object {
    $fields = $_ -split "`t", 2
    [pscustomobject]@{ package = $fields[0]; binary = $fields[1] }
})
$CliBinary = [string]($ReleaseTools | Where-Object package -eq "a4n-cli" | Select-Object -ExpandProperty binary)
if ([string]::IsNullOrWhiteSpace($CliBinary)) {
    throw "release tool catalog is missing a4n-cli"
}
$TempParent = Join-Path $ProjectRoot "target\tmp"
New-Item -ItemType Directory -Force -Path $TempParent | Out-Null
$TempRoot = Join-Path $TempParent ("a4n-release-smoke-" + [guid]::NewGuid().ToString("N"))
$InstallRoot = Join-Path $TempRoot "安装 前缀"
$LauncherOut = Join-Path $TempRoot "launcher.out.log"
$LauncherErr = Join-Path $TempRoot "launcher.err.log"
$Launcher = $null
$PackageRoot = $null
$LocalCli = $null
$LocalConfig = $null
$LocalServer = $null

function Stop-SmokeProcesses {
    if ($null -ne $LocalCli -and $null -ne $LocalConfig -and $null -ne $LocalServer -and (Test-Path -LiteralPath $LocalCli)) {
        & $LocalCli --config $LocalConfig --server $LocalServer local stop --force *> $null
    }
    if ($null -ne $Launcher -and -not $Launcher.HasExited) {
        Stop-Process -Id $Launcher.Id -Force -ErrorAction SilentlyContinue
        $Launcher.WaitForExit(5000) | Out-Null
    }
}

try {
    New-Item -ItemType Directory -Force -Path $InstallRoot | Out-Null
    $entries = & tar.exe -tzf $Archive
    if ($LASTEXITCODE -ne 0) {
        throw "unable to list release archive"
    }
    foreach ($entry in $entries) {
        if ([IO.Path]::IsPathRooted($entry) -or ($entry -split "[/\\]") -contains "..") {
            throw "release archive contains unsafe path: $entry"
        }
    }
    Push-Location -LiteralPath $InstallRoot
    try {
        & tar.exe -xzf $Archive
        if ($LASTEXITCODE -ne 0) {
            throw "unable to extract release archive"
        }
    } finally {
        Pop-Location
    }

    $packageRoots = @(Get-ChildItem -LiteralPath $InstallRoot -Directory)
    if ($packageRoots.Count -ne 1) {
        throw "release archive must contain exactly one package root"
    }
    $PackageRoot = $packageRoots[0].FullName
    $StartScript = Join-Path $PackageRoot "start.ps1"
    if (-not (Test-Path -LiteralPath $StartScript -PathType Leaf)) {
        throw "release package is missing start.ps1"
    }
    foreach ($entry in $ReleaseTools) {
        $tool = "$($entry.binary).exe"
        $toolPath = Join-Path $PackageRoot "bin\$tool"
        if (-not (Test-Path -LiteralPath $toolPath -PathType Leaf)) {
            throw "release package is missing executable tool: $tool"
        }
    }
    $LocalCli = (Resolve-Path -LiteralPath (Join-Path $PackageRoot "bin\$($CliBinary).exe")).Path
    $LocalConfig = Join-Path $TempRoot "cli.json"

    $modules = @($Services | ForEach-Object { $_.name })
    $addrNames = @($Services | ForEach-Object { $_.addr_env })
    $urlNames = @($Services | ForEach-Object { $_.url_env })
    $healthPaths = @($Services | ForEach-Object { $_.health_path })
    $basePort = if ($env:A4N_RELEASE_SMOKE_BASE_PORT) { [int]$env:A4N_RELEASE_SMOKE_BASE_PORT } else { 48641 }
    $maxBasePort = 65535 - $Services.Count
    if ($basePort -lt 1024 -or $basePort -gt $maxBasePort) {
        throw "invalid A4N_RELEASE_SMOKE_BASE_PORT: $basePort"
    }
    $LocalServer = "http://127.0.0.1:$($basePort + $Services.Count)"
    Push-Location -LiteralPath $TempRoot
    try {
        $capabilities = (& $LocalCli --config $LocalConfig --agent capabilities export) | ConvertFrom-Json
        $capabilityData = $capabilities.data
        if ($LASTEXITCODE -ne 0 -or
            $capabilityData.total_commands -ne $capabilityData.matched_commands -or
            $capabilityData.total_commands -ne @($capabilityData.commands).Count) {
            throw "installed CLI capability export failed"
        }
    } finally {
        Pop-Location
    }
    $urls = @()
    for ($index = 0; $index -lt $modules.Count; $index++) {
        $port = $basePort + $index
        $url = "http://127.0.0.1:$port"
        [Environment]::SetEnvironmentVariable($addrNames[$index], "127.0.0.1:$port", "Process")
        [Environment]::SetEnvironmentVariable($urlNames[$index], $url, "Process")
        $urls += $url
        $portInUse = $false
        try {
            Invoke-RestMethod -Uri "$url$($healthPaths[$index])" -TimeoutSec 1 | Out-Null
            $portInUse = $true
        } catch {
        }
        if ($portInUse) {
            throw "release smoke port is already in use: $port"
        }
    }

    $arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$StartScript`""
    $Launcher = Start-Process -FilePath "powershell.exe" -ArgumentList $arguments -WorkingDirectory $PackageRoot `
        -RedirectStandardOutput $LauncherOut -RedirectStandardError $LauncherErr -PassThru

    $ready = $false
    for ($attempt = 0; $attempt -lt 300; $attempt++) {
        $ready = $true
        for ($index = 0; $index -lt $modules.Count; $index++) {
            try {
                $health = Invoke-RestMethod -Uri "$($urls[$index])$($healthPaths[$index])" -TimeoutSec 1
                if ($health.service -ne $modules[$index] -or $health.status -ne "healthy") {
                    $ready = $false
                    break
                }
            } catch {
                $ready = $false
                break
            }
        }
        $Launcher.Refresh()
        if ($Launcher.HasExited) {
            throw "release launcher exited before sidecars became healthy"
        }
        if ($ready) {
            break
        }
        Start-Sleep -Milliseconds 100
    }
    if (-not $ready) {
        throw "release sidecars did not become healthy"
    }

    $hostPath = (Resolve-Path -LiteralPath (Join-Path $PackageRoot "bin\a4n-tauri.exe")).Path
    $hostProcess = $null
    for ($attempt = 0; $attempt -lt 300 -and $null -eq $hostProcess; $attempt++) {
        $hostProcess = Get-Process -Name "a4n-tauri" -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $hostPath } | Select-Object -First 1
        if ($null -eq $hostProcess) {
            Start-Sleep -Milliseconds 100
        }
    }
    if ($null -eq $hostProcess) {
        throw "release host did not start"
    }

    $serviceLogs = @()
    for ($attempt = 0; $attempt -lt 300 -and $serviceLogs.Count -eq 0; $attempt++) {
        $serviceLogs = @(Get-ChildItem -LiteralPath (Join-Path $PackageRoot "data\logs\services") -File -Filter "*.log.*" -ErrorAction SilentlyContinue | Where-Object {
            try {
                -not [string]::IsNullOrWhiteSpace([string](Get-Content -LiteralPath $_.FullName -TotalCount 1 -ErrorAction Stop))
            } catch {
                $false
            }
        })
        if ($serviceLogs.Count -eq 0) {
            Start-Sleep -Milliseconds 100
        }
    }
    if ($serviceLogs.Count -eq 0) {
        throw "release sidecars did not create controlled logs"
    }

    Stop-Process -Id $hostProcess.Id
    if (-not $Launcher.WaitForExit(30000)) {
        throw "release launcher did not exit after host shutdown"
    }
    $Launcher = $null

    for ($index = 0; $index -lt $urls.Count; $index++) {
        $portAlive = $false
        try {
            Invoke-RestMethod -Uri "$($urls[$index])$($healthPaths[$index])" -TimeoutSec 1 | Out-Null
            $portAlive = $true
        } catch {
        }
        if ($portAlive) {
            throw "sidecar port survived host shutdown: $($urls[$index])"
        }
    }

    Push-Location -LiteralPath $TempRoot
    try {
        & $LocalCli --config $LocalConfig --server $LocalServer --agent local start --no-qdrant
        if ($LASTEXITCODE -ne 0) { throw "installed CLI local start failed" }
        & $LocalCli --config $LocalConfig --server $LocalServer --agent health
        if ($LASTEXITCODE -ne 0) { throw "installed CLI health failed" }
        & $LocalCli --config $LocalConfig --server $LocalServer --agent capabilities check
        if ($LASTEXITCODE -ne 0) { throw "installed CLI capability compatibility check failed" }
        & $LocalCli --config $LocalConfig --server $LocalServer --agent local stop
        if ($LASTEXITCODE -ne 0) { throw "installed CLI local stop failed" }
    } finally {
        Pop-Location
    }
    for ($index = 0; $index -lt $urls.Count; $index++) {
        $portAlive = $false
        try {
            Invoke-RestMethod -Uri "$($urls[$index])$($healthPaths[$index])" -TimeoutSec 1 | Out-Null
            $portAlive = $true
        } catch {
        }
        if ($portAlive) {
            throw "local managed sidecar survived local stop: $($urls[$index])"
        }
    }
    $serverAlive = $false
    try {
        Invoke-WebRequest -UseBasicParsing -Uri "$LocalServer/api/health" -TimeoutSec 1 | Out-Null
        $serverAlive = $true
    } catch {
    }
    if ($serverAlive) {
        throw "local Server port survived local stop: $LocalServer"
    }

    Write-Host "windows release package smoke passed: host + CLI tools + local Server + $($Services.Count) sidecars, logs, cleanup, non-ASCII path"
} catch {
    if (Test-Path -LiteralPath $LauncherOut) { Get-Content -LiteralPath $LauncherOut -Tail 200 -ErrorAction SilentlyContinue }
    if (Test-Path -LiteralPath $LauncherErr) { Get-Content -LiteralPath $LauncherErr -Tail 200 -ErrorAction SilentlyContinue }
    throw
} finally {
    Stop-SmokeProcesses
    if ($env:A4N_RELEASE_SMOKE_ARTIFACT_DIR) {
        New-Item -ItemType Directory -Force -Path $env:A4N_RELEASE_SMOKE_ARTIFACT_DIR | Out-Null
        Copy-Item -LiteralPath $LauncherOut, $LauncherErr -Destination $env:A4N_RELEASE_SMOKE_ARTIFACT_DIR -ErrorAction SilentlyContinue
        if ($null -ne $PackageRoot -and (Test-Path -LiteralPath (Join-Path $PackageRoot "data\logs"))) {
            Copy-Item -LiteralPath (Join-Path $PackageRoot "data\logs") -Destination $env:A4N_RELEASE_SMOKE_ARTIFACT_DIR -Recurse -Force
        }
    }
    if ($env:A4N_RELEASE_SMOKE_KEEP_TMP -eq "1") {
        Write-Warning "release smoke files kept at $TempRoot"
    } else {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
