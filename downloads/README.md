# Android test build

[Download Project Garage for Android (ARM64 debug APK)](project-garage-arm64-debug.apk)

This is a signed development build for ARM64 Android devices, not a Play Store release. Install it, enter your name, and open your garage. Cars, parts, purchase records, receipts, and progress photos are stored directly on your device through Rust and SQLite. The personal garage works offline and requires no account, local server, computer, or server address.

Sharing is optional. A shared garage uses a hosted HTTPS connection for owner/staff access across devices. The personal device garage stays separate and is not uploaded automatically.

This build adds System/Light/Dark themes, larger mobile controls, car source and initial purchase cost, and automatic desktop discovery of Android apps on the same Wi-Fi. Keep Android open and allow its local-network permission when prompted. Discovery does not transfer or merge garage records.

The browser interface has been exercised with car creation, purchase records, spending, completion, and photo journal uploads. Rust tests verify that device records, receipts, and journal photos survive reopening with no server running. Testing on a physical Android device remains to be done.

To build locally on Windows, run `./scripts/build-android.ps1` from the repository root. Future test builds are also available as artifacts of the **Android APK** GitHub Actions workflow when that workflow succeeds.
