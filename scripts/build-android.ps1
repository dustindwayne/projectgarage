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
$taskBuildLog = Join-Path $env:TEMP 'projectgarage-android-build.log'
npm.cmd run tauri android build -- --debug --apk --target aarch64 --ci 2>&1 | Tee-Object -FilePath $taskBuildLog
if ($LASTEXITCODE -ne 0) {
    $taskNativeLib = Join-Path $taskRoot 'target\aarch64-linux-android\debug\libproject_garage_lib.so'
    if ((Get-Content -LiteralPath $taskBuildLog -Raw) -notmatch 'Creation symbolic link is not allowed' -or -not (Test-Path -LiteralPath $taskNativeLib)) {
        throw 'Android build failed. See the build output above.'
    }
    Write-Output 'Rust compiled successfully. Packaging with a file copy because Windows disallows symbolic links.'
    $taskJni = Join-Path $taskRoot 'src-tauri\gen\android\app\src\main\jniLibs\arm64-v8a'
    New-Item -ItemType Directory -Force -Path $taskJni | Out-Null
    Copy-Item -LiteralPath $taskNativeLib -Destination (Join-Path $taskJni 'libproject_garage_lib.so') -Force
    $taskStrip = Join-Path $env:NDK_HOME 'toolchains\llvm\prebuilt\windows-x86_64\bin\llvm-strip.exe'
    & $taskStrip --strip-unneeded (Join-Path $taskJni 'libproject_garage_lib.so')
    if ($LASTEXITCODE -ne 0) { throw 'Could not remove native debug-symbol bulk.' }
    $taskApkOutput = Join-Path $taskRoot 'src-tauri\gen\android\app\build\outputs\apk\arm64\debug'
    if (Test-Path -LiteralPath $taskApkOutput) {
        # Remove only generated APK files so incremental ZIP padding does not bloat the download.
        Get-ChildItem -LiteralPath $taskApkOutput -Filter '*.apk' | ForEach-Object { Remove-Item -LiteralPath $_.FullName }
    }
    Push-Location 'src-tauri\gen\android'
    try {
        .\gradlew.bat assembleArm64Debug -x rustBuildArm64Debug --no-daemon
        if ($LASTEXITCODE -ne 0) { throw 'Android packaging failed' }
    } finally { Pop-Location }
}
New-Item -ItemType Directory -Force -Path 'artifacts' | Out-Null
$taskApk = Get-ChildItem 'src-tauri\gen\android\app\build\outputs\apk' -Recurse -Filter '*.apk' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $taskApk) { throw 'The build completed without producing an APK.' }
Copy-Item -LiteralPath $taskApk.FullName -Destination 'artifacts\project-garage-arm64-debug.apk' -Force
Write-Output (Join-Path $taskRoot 'artifacts\project-garage-arm64-debug.apk')
