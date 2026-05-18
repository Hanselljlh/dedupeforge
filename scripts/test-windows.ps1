$ErrorActionPreference = "Stop"

$env:HOME = $env:USERPROFILE
$env:PATH = "C:\msys64\ucrt64\bin;C:\msys64\usr\bin;$env:PATH"
$env:CARGO_TARGET_DIR = "C:\dedupeforge-target"
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = "C:\msys64\ucrt64\bin\gcc.exe"
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_AR = "C:\msys64\ucrt64\bin\gcc-ar.exe"

& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-x86_64-pc-windows-gnu test
