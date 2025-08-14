param(
    [string]$Root = ".",
    [string[]]$Extensions = @('.rs','.ts','.tsx','.vue','.cs','.jsx'),
    [string[]]$ExcludeDirs = @('node_modules','target','dist','.git','build','out')
)

function Test-ExcludedPath($fullName, $excludePatterns) {
    foreach ($pat in $excludePatterns) { if ($fullName -like $pat) { return $true } }
    return $false
}

$excludePatterns = $ExcludeDirs | ForEach-Object { "*\$_\*" }

$items = Get-ChildItem -Path $Root -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { ($Extensions -contains $_.Extension) -and -not (Test-ExcludedPath $_.FullName $excludePatterns) }

$results = @()
foreach ($it in $items) {
    try {
        $lineCount = (Get-Content -LiteralPath $it.FullName -ErrorAction Stop | Measure-Object -Line).Lines
    } catch {
        $lineCount = 0
    }
    $rel = Resolve-Path -Relative -Path $it.FullName
    $results += [PSCustomObject]@{ Lines = $lineCount; Path = $rel }
}

$results = $results | Sort-Object Lines -Descending

$csvPath = Join-Path (Split-Path -Parent $PSCommandPath) 'linecounts.csv'
$results | Export-Csv -NoTypeInformation -Encoding UTF8 -Path $csvPath

# 输出前 50 行到控制台
$results | Select-Object -First 50 | Format-Table -AutoSize
