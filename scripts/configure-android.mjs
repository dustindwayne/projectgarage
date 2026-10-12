import { readFileSync, writeFileSync, copyFileSync } from 'node:fs';
const manifest = 'src-tauri/gen/android/app/src/main/AndroidManifest.xml';
let xml = readFileSync(manifest, 'utf8');
if (!xml.includes('android.permission.ACCESS_LOCAL_NETWORK'))
  xml = xml.replace(
    '<uses-permission android:name="android.permission.INTERNET" />',
    '<uses-permission android:name="android.permission.INTERNET" />\n    <uses-permission android:name="android.permission.ACCESS_LOCAL_NETWORK" />',
  );
writeFileSync(manifest, xml);
if (!xml.includes('android.permission.REQUEST_INSTALL_PACKAGES')) {
  xml = xml.replace(
    '<uses-permission android:name="android.permission.INTERNET" />',
    '<uses-permission android:name="android.permission.INTERNET" />\n    <uses-permission android:name="android.permission.REQUEST_INSTALL_PACKAGES" />',
  );
  writeFileSync(manifest, xml);
}
copyFileSync(
  'src-tauri/android/GarageUpdatesPlugin.kt',
  'src-tauri/gen/android/app/src/main/java/com/projectgarage/mobile/GarageUpdatesPlugin.kt',
);
const activity = 'src-tauri/gen/android/app/src/main/java/com/projectgarage/mobile/MainActivity.kt';
let source = readFileSync(activity, 'utf8');
if (!source.includes('projectGarageDiscoveryPermission')) {
  const method = `
    // Android 17 requires explicit user permission before Wi-Fi discovery can announce this app.
    private fun projectGarageDiscoveryPermission() {
        if (android.os.Build.VERSION.SDK_INT >= 37 && checkSelfPermission("android.permission.ACCESS_LOCAL_NETWORK") != android.content.pm.PackageManager.PERMISSION_GRANTED) {
            requestPermissions(arrayOf("android.permission.ACCESS_LOCAL_NETWORK"), 39073)
        }
    }
`;
  if (source.includes('override fun onCreate'))
    source = source.replace(
      'super.onCreate(savedInstanceState)',
      'super.onCreate(savedInstanceState)\n    projectGarageDiscoveryPermission()',
    );
  else
    source = source.replace(
      /}\s*$/,
      `override fun onCreate(savedInstanceState: android.os.Bundle?) { super.onCreate(savedInstanceState); projectGarageDiscoveryPermission() }\n}`,
    );
  source = source.replace(/}\s*$/, method + '\n}');
}
writeFileSync(activity, source);
