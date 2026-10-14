$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path $PSScriptRoot -Parent
$taskManifest = Get-Content -LiteralPath (Join-Path $taskRoot 'downloads\latest.json') -Raw | ConvertFrom-Json
$taskPackages = @($taskManifest.platforms.PSObject.Properties | ForEach-Object {$_.Value}) + @($taskManifest.tools.agent_bridge)
foreach ($taskPackage in $taskPackages) {
    if ($taskPackage.path -notin @('downloads/project-garage-arm64-debug.apk','downloads/project-garage-windows.exe','downloads/project-garage-agent.exe')) { throw 'Unexpected installer path' }
    $taskPath = Join-Path $taskRoot $taskPackage.path
    if ((Get-FileHash -LiteralPath $taskPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $taskPackage.sha256 -or (Get-Item -LiteralPath $taskPath).Length -ne $taskPackage.size) { throw 'An installer does not match its manifest; build before publishing.' }
}
Write-Output ('Verified Project Garage '+$taskManifest.version+' installers and agent bridge.')
