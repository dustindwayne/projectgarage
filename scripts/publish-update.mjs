import { readFileSync, writeFileSync, copyFileSync, existsSync } from 'node:fs';
import { createHash } from 'node:crypto';
const version = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')).version;
const platforms = {};
for (const [key, source, target] of [
  [
    'android-aarch64',
    'artifacts/project-garage-arm64-debug.apk',
    'downloads/project-garage-arm64-debug.apk',
  ],
  ['windows-x86_64', 'target/release/project-garage.exe', 'downloads/project-garage-windows.exe'],
]) {
  if (!existsSync(source)) throw new Error(`Build ${source} before publishing an update`);
  copyFileSync(source, target);
  const bytes = readFileSync(target);
  const sha256 = createHash('sha256').update(bytes).digest('hex');
  platforms[key] = { path: target, sha256, size: bytes.length };
  writeFileSync(`${target}.sha256`, `${sha256}  ${target.split('/').pop()}\n`);
}
writeFileSync(
  'downloads/latest.json',
  JSON.stringify(
    {
      version,
      notes: 'Token-free public updates, new accent-aware logo, true black and charcoal dark mode, and phone-first project screens.',
      platforms,
    },
    null,
    2,
  ) + '\n',
);
console.log(
  `Prepared update channel ${version}. Commit and push downloads/ with its matching app version to main.`,
);
