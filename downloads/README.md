# Android test build

[Download Project Garage for Android (ARM64 debug APK)](project-garage-arm64-debug.apk)

This is a signed development build for ARM64 Android devices, not a Play Store release. The app includes the garage interface and connects to the Rust garage service for accounts, sharing, and updates. Set your service address under **Garage connection** on the welcome screen. Use HTTPS for a shared installation. Service hosting is not included with the APK.

The browser interface has been exercised with car creation, purchase records, spending, completion, and photo journal uploads. The APK has been built and its signature checked; testing on a physical Android device remains to be done.

To build locally on Windows, run `./scripts/build-android.ps1` from the repository root. Future test builds are also available as artifacts of the **Android APK** GitHub Actions workflow when that workflow succeeds.
