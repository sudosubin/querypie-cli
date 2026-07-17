# Installer for querypie-cli on Windows.
#
#   powershell -c "irm https://raw.githubusercontent.com/sudosubin/querypie-cli/main/scripts/install.ps1 | iex"
#
# Environment variables:
#   QUERYPIE_VERSION      Release tag to install (default: latest, e.g. v0.1.1)
#   QUERYPIE_INSTALL_DIR  Where to install the binary (default: %LOCALAPPDATA%\querypie-cli\bin)

$ErrorActionPreference = 'Stop'

$repo = 'sudosubin/querypie-cli'
$version = if ($env:QUERYPIE_VERSION) { $env:QUERYPIE_VERSION } else { 'latest' }
$dir = if ($env:QUERYPIE_INSTALL_DIR) { $env:QUERYPIE_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'querypie-cli\bin' }

$target = switch ($env:PROCESSOR_ARCHITECTURE) {
    'AMD64' { 'x86_64-pc-windows-msvc' }
    'ARM64' { 'aarch64-pc-windows-msvc' }
    default { throw "querypie-cli: unsupported architecture $env:PROCESSOR_ARCHITECTURE" }
}

$archive = "querypie-cli-$target.zip"
$base = if ($version -eq 'latest') {
    "https://github.com/$repo/releases/latest/download"
} else {
    "https://github.com/$repo/releases/download/$version"
}

$tmp = New-Item -ItemType Directory -Path (Join-Path ([IO.Path]::GetTempPath()) ([IO.Path]::GetRandomFileName()))
try {
    Write-Host "Downloading querypie ($target)..."
    Invoke-WebRequest "$base/$archive" -OutFile "$tmp\$archive"
    Invoke-WebRequest "$base/$archive.sha256" -OutFile "$tmp\$archive.sha256"

    $expected = (Get-Content "$tmp\$archive.sha256" -Raw).Split()[0]
    $actual = (Get-FileHash "$tmp\$archive" -Algorithm SHA256).Hash
    if ($expected -ne $actual) { throw 'querypie-cli: checksum mismatch' }

    Expand-Archive "$tmp\$archive" -DestinationPath $tmp -Force
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    Copy-Item "$tmp\querypie-cli-$target\querypie.exe" (Join-Path $dir 'querypie.exe') -Force
    Write-Host "Installed querypie.exe to $dir"

    if (($env:Path -split ';') -notcontains $dir) {
        Write-Warning "$dir is not in your PATH"
    }
}
finally {
    Remove-Item -Recurse -Force $tmp
}
