$ErrorActionPreference = 'Stop'
$env:BRIDGE_TICKS = 12
# Ensure old logs are cleared
Remove-Item -ErrorAction SilentlyContinue -Force "bridge.admin.out.jsonl","bridge.admin.err.txt"
# Run bridge and redirect output
& dotnet "bin\Release\net8.0\sensor-bridge.dll" 1> "bridge.admin.out.jsonl" 2> "bridge.admin.err.txt"
