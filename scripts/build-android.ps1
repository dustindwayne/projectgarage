$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $taskRoot
if (-not $env:ANDROID_HOME) { $env:ANDROID_HOME = Join-Path $env:LOCALAPPDATA 'Android\Sdk' }
if (-not $env:JAVA_HOME -or -not (Test-Path (Join-Path $env:JAVA_HOME 'bin\javac.exe'))) {
    $env:JAVA_HOME = 'C:\Program Files\Android\Android Studio\jbr'
}
if (-not $env:NDK_HOME) { $env:NDK_HOME = Join-Path $env:ANDROID_HOME 'ndk\28.2.13676358' }
if (-not (Test-Path (Join-Path $env:NDK_HOME 'source.properties'))) { throw 'Install Android NDK 28.2.13676358 or set NDK_HOME to your installed NDK.' }
if (-not (Test-Path (Join-Path $env:ANDROID_HOME 'cmdline-tools\latest\bin\sdkmanager.bat'))) { throw 'Install Android SDK command-line tools in Android Studio.' }
if (-not (Test-Path 'src-tauri\gen\android\gradlew.bat')) {
    npm.cmd run tauri android init -- --skip-targets-install --ci
    if ($LASTEXITCODE -ne 0) { throw 'Android initialization failed' }
}
npm.cmd run tauri android build -- --debug --apk --target aarch64 --ci
if ($LASTEXITCODE -ne 0) { throw 'Android build failed' }
New-Item -ItemType Directory -Force -Path 'artifacts' | Out-Null
$taskApk = Get-ChildItem 'src-tauri\gen\android\app\build\outputs\apk' -Recurse -Filter '*.apk' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $taskApk) { throw 'The build completed without producing an APK.' }
Copy-Item -LiteralPath $taskApk.FullName -Destination 'artifacts\project-garage-arm64-debug.apk' -Force
Write-Output (Join-Path $taskRoot 'artifacts\project-garage-arm64-debug.apk')
