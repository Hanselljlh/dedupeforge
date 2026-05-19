$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$distRoot = Join-Path $repoRoot "dist"
$distDir = Join-Path $distRoot "windows-x86_64-gnu"
$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
$targetDir = "C:\dedupeforge-target"
$releaseDir = Join-Path $targetDir "release"

$env:HOME = $env:USERPROFILE
$env:PATH = "C:\msys64\ucrt64\bin;C:\msys64\usr\bin;$env:PATH"
$env:CARGO_TARGET_DIR = $targetDir
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = "C:\msys64\ucrt64\bin\gcc.exe"
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_AR = "C:\msys64\ucrt64\bin\gcc-ar.exe"

New-Item -ItemType Directory -Force -Path $distDir | Out-Null

& $cargoBin +stable-x86_64-pc-windows-gnu build --release --bin dedupeforge --bin dedupeforge-gui

Copy-Item -Force (Join-Path $releaseDir "dedupeforge.exe") $distDir
Copy-Item -Force (Join-Path $releaseDir "dedupeforge-gui.exe") $distDir
Copy-Item -Force (Join-Path $repoRoot "README.md") $distDir
if (Test-Path (Join-Path $repoRoot "LICENSE-MIT")) {
    Copy-Item -Force (Join-Path $repoRoot "LICENSE-MIT") $distDir
}
if (Test-Path (Join-Path $repoRoot "LICENSE-APACHE")) {
    Copy-Item -Force (Join-Path $repoRoot "LICENSE-APACHE") $distDir
}

$releaseNotes = @"
DedupeForge Windows Release Bundle
================================

Files:
- dedupeforge.exe      CLI application
- dedupeforge-gui.exe  Desktop GUI application

Built with:
- Rust stable-x86_64-pc-windows-gnu
- Output target dir: $targetDir
"@

Set-Content -Path (Join-Path $distDir "BUILD_INFO.txt") -Value $releaseNotes

Write-Host "Release bundle created at: $distDir"
