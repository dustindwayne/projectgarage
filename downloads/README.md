# Project Garage 0.5.0

[Download Android APK](project-garage-arm64-debug.apk) · [Download Windows app](project-garage-windows.exe)

This update adds on-device receipt-image text recognition with mandatory review, spending statistics and filters, CSV exports, saved-garage import, and encrypted phone-approved Wi-Fi imports to desktop. Android exports use the system share/save chooser.

Install once, then use **Settings → App updates** for future downloads. Updates use the public installer repository without a GitHub account or token. Android opens the system installer for confirmation and checks the existing signing key. Source remains private.

The Android ARM64 APK is a signed development build. Personal garage records stay in SQLite on your device; no account, local web server, or computer is required. Optional shared garages use a hosted HTTPS service.

For Wi-Fi imports, open Settings on both devices, select the phone on desktop, verify the matching code, approve on the phone, and review the desktop import. Import adds missing records and preserves existing records. No continuous synchronization is included.

Twenty-two Rust tests and receipt-parser checks pass. Receipt scanning was exercised with a synthetic receipt in the browser. Physical Android testing is still required, including share/save and Wi-Fi transfers.

Build Android on Windows using scripts/build-android.ps1.
