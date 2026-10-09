param(
    [Parameter(Mandatory = $true)]
    [string]$Msi
)

$ErrorActionPreference = "Stop"
if (-not (Test-Path -LiteralPath $Msi -PathType Leaf)) {
    throw "Windows MSI installer does not exist: $Msi"
}

$ProjectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$ManifestTool = Join-Path $ProjectRoot "scripts\release_manifest.rb"
$ToolRows = @(& ruby $ManifestTool tool-rows)
if ($LASTEXITCODE -ne 0) { throw "unable to read release tool catalog" }
$Tools = @($ToolRows | ForEach-Object { ($_ -split "`t", 2)[1] })
$ServiceRows = @(& ruby $ManifestTool service-rows)
if ($LASTEXITCODE -ne 0) { throw "unable to read sidecar catalog" }
$Services = @($ServiceRows | ForEach-Object { ($_ -split "`t", 5)[1] })

$TempParent = Join-Path $ProjectRoot "target\tmp"
New-Item -ItemType Directory -Force -Path $TempParent | Out-Null
$TempRoot = Join-Path $TempParent ("a4n-msi-layout-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null
try {
    $Msi = (Resolve-Path -LiteralPath $Msi).Path
    $arguments = @("/a", "`"$Msi`"", "/qn", "TARGETDIR=`"$TempRoot`"")
    $process = Start-Process -FilePath "msiexec.exe" -ArgumentList $arguments -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        throw "administrative MSI extraction failed with exit code $($process.ExitCode)"
    }

    $files = @(Get-ChildItem -LiteralPath $TempRoot -Recurse -File)
    $required = @("a4n-tauri.exe", "onnxruntime.dll", "a4n_db.dll")
    foreach ($name in $required) {
        if (-not ($files | Where-Object Name -eq $name | Select-Object -First 1)) {
            throw "Windows MSI is missing $name"
        }
    }
    foreach ($name in @($Tools + $Services)) {
        $executable = "$($name).exe"
        if (-not ($files | Where-Object Name -eq $executable | Select-Object -First 1)) {
            throw "Windows MSI is missing executable $executable"
        }
    }
    if (-not ($files | Where-Object { $_.Name -match "^(lib)?std-.+\.dll$" } | Select-Object -First 1)) {
        throw "Windows MSI is missing the Rust shared standard library"
    }
    if (-not ($files | Where-Object Name -eq "_registry.yaml" | Select-Object -First 1)) {
        throw "Windows MSI is missing prompts/_registry.yaml"
    }
    Write-Output "Windows MSI installer layout verified: $(Split-Path -Leaf $Msi)"
} finally {
    Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
