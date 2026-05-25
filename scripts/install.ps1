$ErrorActionPreference = 'Stop'

$Repo = 'YinMo19/sql-web'
$Version = if ($env:SQL_WEB_VERSION) { $env:SQL_WEB_VERSION } else { 'latest' }
$InstallDir = if ($env:SQL_WEB_INSTALL_DIR) { $env:SQL_WEB_INSTALL_DIR } else { Join-Path $env:USERPROFILE '.sql-web\bin' }

$Arch = switch ($env:PROCESSOR_ARCHITECTURE) {
    'AMD64' { 'x86_64' }
    'ARM64' { 'aarch64' }
    default { throw "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE" }
}

$Target = "$Arch-pc-windows-msvc"
$Asset = "sql-web-$Target.zip"
if ($Version -eq 'latest') {
    $Url = "https://github.com/$Repo/releases/latest/download/$Asset"
} else {
    $Url = "https://github.com/$Repo/releases/download/$Version/$Asset"
}

$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("sql-web-install-" + [System.Guid]::NewGuid())
New-Item -ItemType Directory -Force -Path $TempDir | Out-Null

try {
    $Archive = Join-Path $TempDir $Asset
    Invoke-WebRequest -Uri $Url -OutFile $Archive -UseBasicParsing

    Expand-Archive -Path $Archive -DestinationPath $TempDir -Force
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

    $Binary = Join-Path $TempDir "sql-web-$Target\sql-web.exe"
    if (!(Test-Path $Binary)) {
        throw "sql-web.exe not found in release archive"
    }

    Copy-Item $Binary (Join-Path $InstallDir 'sql-web.exe') -Force

    $UserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $PathEntries = $UserPath -split ';' | Where-Object { $_ -ne '' }
    if ($PathEntries -notcontains $InstallDir) {
        $NewPath = if ($UserPath) { "$UserPath;$InstallDir" } else { $InstallDir }
        [Environment]::SetEnvironmentVariable('Path', $NewPath, 'User')
        Write-Host "Added $InstallDir to your user PATH. Restart your terminal to use sql-web from any directory."
    }

    Write-Host "sql-web installed to $(Join-Path $InstallDir 'sql-web.exe')"
} finally {
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
}
