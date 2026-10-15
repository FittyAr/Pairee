# Runs makeappx.exe from the newest Windows SDK with the given arguments
# (`pack` in package-msix.ps1, `bundle` in msstore-publish.ps1); falls back
# to the one on the PATH.
$ErrorActionPreference = 'Stop'

$makeappx = Get-ChildItem -Path 'C:\Program Files (x86)\Windows Kits\10\bin' -Filter 'makeappx.exe' -Recurse -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match '\\x64\\' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1 -ExpandProperty FullName
if (-not $makeappx) {
    $makeappx = 'makeappx.exe'
}
& $makeappx @args
if ($LASTEXITCODE -ne 0) {
    throw "makeappx $($args[0]) failed with exit code $LASTEXITCODE"
}
