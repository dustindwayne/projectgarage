$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path $PSScriptRoot -Parent
$taskCheckout = Join-Path $taskRoot 'artifacts\public-updates'
$taskRemote = 'https://github.com/dustindwayne/projectgarage-updates.git'
$taskFiles = @('latest.json','project-garage-arm64-debug.apk','project-garage-arm64-debug.apk.sha256','project-garage-windows.exe','project-garage-windows.exe.sha256','project-garage-agent.exe','project-garage-agent-skill.zip','project-garage-agent-skill.md','project-garage-agent-actions.md','project-garage-agent-bootstrap.ps1')
$taskManifest = Get-Content -LiteralPath (Join-Path $taskRoot 'downloads\latest.json') -Raw | ConvertFrom-Json
$taskPackages = @($taskManifest.platforms.PSObject.Properties | ForEach-Object {$_.Value}) + @($taskManifest.tools.agent_bridge)
foreach ($taskPackage in $taskPackages) {
    if ($taskPackage.path -notin @('downloads/project-garage-arm64-debug.apk','downloads/project-garage-windows.exe','downloads/project-garage-agent.exe')) { throw 'Unexpected installer path' }
    $taskPath = Join-Path $taskRoot $taskPackage.path
    if ((Get-FileHash -LiteralPath $taskPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $taskPackage.sha256 -or (Get-Item -LiteralPath $taskPath).Length -ne $taskPackage.size) { throw 'An installer does not match its manifest; build before publishing.' }
}
if (-not (Test-Path -LiteralPath (Join-Path $taskCheckout '.git'))) {
    New-Item -ItemType Directory -Force -Path $taskCheckout | Out-Null
    git -C $taskCheckout init -b main
    git -C $taskCheckout remote add origin $taskRemote
    if ($LASTEXITCODE -ne 0) { throw 'Could not initialize the installer repository' }
} else {
    if ((git -C $taskCheckout remote get-url origin) -ne $taskRemote) { throw 'Unexpected publication remote' }
    if ((git -C $taskCheckout branch --show-current) -ne 'main') { throw 'Updates must be published on main' }
    git -C $taskCheckout pull --ff-only origin main
    if ($LASTEXITCODE -ne 0) { throw 'Could not refresh the public update channel' }
}
$taskOutput = Join-Path $taskCheckout 'downloads'
git -C $taskCheckout config user.name dustindwayne
git -C $taskCheckout config user.email dustindwayne@users.noreply.github.com
New-Item -ItemType Directory -Force -Path $taskOutput | Out-Null
foreach ($taskFile in $taskFiles) { Copy-Item -LiteralPath (Join-Path $taskRoot "downloads\$taskFile") -Destination (Join-Path $taskOutput $taskFile) -Force }
$taskReadme = @('# Project Garage downloads','','Public installers, update manifest, and optional external-agent skill. App source remains private.','','- [Android APK](downloads/project-garage-arm64-debug.apk)','- [Windows app](downloads/project-garage-windows.exe)','- [External-agent skill](downloads/project-garage-agent-skill.zip)','- [Skill instructions](downloads/project-garage-agent-skill.md)','','Install the newest app once. Settings → App updates needs no GitHub account or token.','','The external-agent skill controls the local desktop garage through a verified command bridge. Android uses a context-file handoff. No AI provider setup is required in the app.','','Android packages are development APKs. Android asks you to confirm installation. Garage records stay in separate app storage.') -join [Environment]::NewLine
Set-Content -LiteralPath (Join-Path $taskCheckout 'README.md') -Value $taskReadme -Encoding utf8
$taskAllowedPaths = @('README.md') + @($taskFiles | ForEach-Object {'downloads/'+$_})
$taskPending = git -C $taskCheckout status --porcelain --untracked-files=all
foreach ($taskLine in $taskPending) {if ($taskLine.Substring(3) -notin $taskAllowedPaths) {throw 'Unexpected public file. Publication stopped.'}}
git -C $taskCheckout add -- @taskAllowedPaths
if ($taskPending) {git -C $taskCheckout commit -m "Publish Project Garage $($taskManifest.version) installers and agent skill";if ($LASTEXITCODE -ne 0) {throw 'Public update commit failed'}}
git -C $taskCheckout push -u origin main
if ($LASTEXITCODE -ne 0) {throw 'Public update push failed'}
