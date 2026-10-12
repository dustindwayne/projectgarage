$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path $PSScriptRoot -Parent
$taskCheckout = Join-Path $taskRoot 'artifacts\public-updates'
$taskRemote = 'https://github.com/dustindwayne/projectgarage-updates.git'
$taskFiles = @('latest.json','project-garage-arm64-debug.apk','project-garage-arm64-debug.apk.sha256','project-garage-windows.exe','project-garage-windows.exe.sha256')
$taskManifest = Get-Content -LiteralPath (Join-Path $taskRoot 'downloads\latest.json') -Raw | ConvertFrom-Json
foreach ($taskPlatform in $taskManifest.platforms.PSObject.Properties) {
    $taskPackage = $taskPlatform.Value
    if ($taskPackage.path -notin @('downloads/project-garage-arm64-debug.apk','downloads/project-garage-windows.exe')) { throw 'Unexpected installer path' }
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
$taskReadme = @'
# Project Garage downloads

Public Android and Windows installers and the update manifest. App source remains private.

- [Android APK](downloads/project-garage-arm64-debug.apk)
- [Windows app](downloads/project-garage-windows.exe)

Install the newest build once. Future updates are available inside Settings → App updates with no GitHub account or token.

Android packages are development APKs. Android will ask you to allow installation from this source and confirm updates. Garage records stay in the app's separate storage.
'@
Set-Content -LiteralPath (Join-Path $taskCheckout 'README.md') -Value $taskReadme -Encoding utf8
$taskPending = git -C $taskCheckout status --porcelain --untracked-files=all
foreach ($taskLine in $taskPending) {
    if ($taskLine.Substring(3) -notmatch '^(README\.md|downloads/(latest\.json|project-garage-arm64-debug\.apk(\.sha256)?|project-garage-windows\.exe(\.sha256)?))$') { throw 'Unexpected public file. Publication stopped.' }
}
git -C $taskCheckout add -- README.md downloads/latest.json downloads/project-garage-arm64-debug.apk downloads/project-garage-arm64-debug.apk.sha256 downloads/project-garage-windows.exe downloads/project-garage-windows.exe.sha256
if ($taskPending) {
    git -C $taskCheckout commit -m "Publish Project Garage $($taskManifest.version) installers"
    if ($LASTEXITCODE -ne 0) { throw 'Public update commit failed' }
}
git -C $taskCheckout push -u origin main
if ($LASTEXITCODE -ne 0) { throw 'Public update push failed' }
