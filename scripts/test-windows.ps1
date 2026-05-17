$ErrorActionPreference = "Stop"

$env:HOME = $env:USERPROFILE
$env:PATH = "C:\msys64\ucrt64\bin;C:\msys64\usr\bin;$env:PATH"

& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-x86_64-pc-windows-gnu test
