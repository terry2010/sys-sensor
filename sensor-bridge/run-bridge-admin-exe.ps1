$ErrorActionPreference = 'Stop'
$env:BRIDGE_TICKS = 12
$exe = Join-Path $PSScriptRoot 'bin\Release\win-x64\publish\sensor-bridge.exe'
if (!(Test-Path $exe)) { throw "sensor-bridge.exe not found at $exe" }
# Clear old logs
Remove-Item -ErrorAction SilentlyContinue -Force "$PSScriptRoot\bridge.admin.out.jsonl","$PSScriptRoot\bridge.admin.err.txt"
& $exe 1> "$PSScriptRoot\bridge.admin.out.jsonl" 2> "$PSScriptRoot\bridge.admin.err.txt"
