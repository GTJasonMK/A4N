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
$TempRoot = Join-Path ([IO.Path]::GetTempPath()) ("a4n-release-smoke-" + [guid]::NewGuid().ToString("N"))
$InstallRoot = Join-Path $TempRoot "安装 前缀"
$LauncherOut = Join-Path $TempRoot "launcher.out.log"
$LauncherErr = Join-Path $TempRoot "launcher.err.log"
$Launcher = $null
$PackageRoot = $null
$SidecarPids = @()

function Stop-SmokeProcesses {
    if ($null -ne $Launcher -and -not $Launcher.HasExited) {
        Stop-Process -Id $Launcher.Id -Force -ErrorAction SilentlyContinue
        $Launcher.WaitForExit(5000) | Out-Null
    }
    if ($null -ne $PackageRoot) {
        Get-ChildItem -LiteralPath (Join-Path $PackageRoot "data\services") -Filter "*.pid" -ErrorAction SilentlyContinue | ForEach-Object {
            $pidValue = 0
            if ([int]::TryParse((Get-Content -LiteralPath $_.FullName -Raw).Trim(), [ref]$pidValue)) {
                Stop-Process -Id $pidValue -Force -ErrorAction SilentlyContinue
            }
        }
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

    $modules = @("prompt", "llm", "embedding", "rag", "image", "video", "import", "export")
    $addrNames = @(
        "A4N_PROMPT_SERVICE_ADDR", "A4N_LLM_GATEWAY_ADDR", "A4N_EMBEDDING_SERVICE_ADDR", "A4N_RAG_SERVICE_ADDR",
        "A4N_IMAGE_SERVICE_ADDR", "A4N_VIDEO_SERVICE_ADDR", "A4N_IMPORT_SERVICE_ADDR", "A4N_EXPORT_SERVICE_ADDR"
    )
    $urlNames = @(
        "A4N_PROMPT_SERVICE_URL", "A4N_LLM_GATEWAY_URL", "A4N_EMBEDDING_SERVICE_URL", "A4N_RAG_SERVICE_URL",
        "A4N_IMAGE_SERVICE_URL", "A4N_VIDEO_SERVICE_URL", "A4N_IMPORT_SERVICE_URL", "A4N_EXPORT_SERVICE_URL"
    )
    $basePort = if ($env:A4N_RELEASE_SMOKE_BASE_PORT) { [int]$env:A4N_RELEASE_SMOKE_BASE_PORT } else { 48641 }
    if ($basePort -lt 1024 -or $basePort -gt 65528) {
        throw "invalid A4N_RELEASE_SMOKE_BASE_PORT: $basePort"
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
            Invoke-RestMethod -Uri "$url/healthz" -TimeoutSec 1 | Out-Null
            $portInUse = $true
        } catch {
        }
        if ($portInUse) {
            throw "release smoke port is already in use: $port"
        }
    }

    $pidDirectory = Join-Path $PackageRoot "data\services"
    New-Item -ItemType Directory -Force -Path $pidDirectory | Out-Null
    Set-Content -LiteralPath (Join-Path $pidDirectory "prompt.pid") -Value "999999" -Encoding ascii

    $arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$StartScript`""
    $Launcher = Start-Process -FilePath "powershell.exe" -ArgumentList $arguments -WorkingDirectory $PackageRoot `
        -RedirectStandardOutput $LauncherOut -RedirectStandardError $LauncherErr -PassThru

    $ready = $false
    for ($attempt = 0; $attempt -lt 300; $attempt++) {
        $ready = $true
        for ($index = 0; $index -lt $modules.Count; $index++) {
            try {
                $health = Invoke-RestMethod -Uri "$($urls[$index])/healthz" -TimeoutSec 1
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

    $pidFiles = @(Get-ChildItem -LiteralPath $pidDirectory -Filter "*.pid")
    if ($pidFiles.Count -ne 8) {
        throw "expected 8 sidecar PID files, found $($pidFiles.Count)"
    }
    foreach ($pidFile in $pidFiles) {
        $pidValue = 0
        if (-not [int]::TryParse((Get-Content -LiteralPath $pidFile.FullName -Raw).Trim(), [ref]$pidValue) -or $pidValue -eq 999999) {
            throw "invalid or stale sidecar PID in $($pidFile.Name)"
        }
        Get-Process -Id $pidValue -ErrorAction Stop | Out-Null
        $SidecarPids += $pidValue
    }
    $serviceLogs = @(Get-ChildItem -LiteralPath (Join-Path $PackageRoot "data\logs\services") -File -Recurse | Where-Object Length -gt 0)
    if ($serviceLogs.Count -eq 0) {
        throw "release sidecars did not create controlled logs"
    }

    Stop-Process -Id $hostProcess.Id
    if (-not $Launcher.WaitForExit(30000)) {
        throw "release launcher did not exit after host shutdown"
    }
    $Launcher = $null

    foreach ($pidValue in $SidecarPids) {
        if (Get-Process -Id $pidValue -ErrorAction SilentlyContinue) {
            throw "sidecar process survived host shutdown: $pidValue"
        }
    }
    if (Get-ChildItem -LiteralPath $pidDirectory -Filter "*.pid" -ErrorAction SilentlyContinue) {
        throw "sidecar PID files survived host shutdown"
    }
    foreach ($url in $urls) {
        $portAlive = $false
        try {
            Invoke-RestMethod -Uri "$url/healthz" -TimeoutSec 1 | Out-Null
            $portAlive = $true
        } catch {
        }
        if ($portAlive) {
            throw "sidecar port survived host shutdown: $url"
        }
    }

    Write-Host "windows release package smoke passed: host + 8 sidecars, logs, stale PID, cleanup, non-ASCII path"
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
