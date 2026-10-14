import { readFileSync, writeFileSync, copyFileSync, existsSync } from 'node:fs';
import { createHash } from 'node:crypto';
const version = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')).version;
const platforms = {};
const tools = {};
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
const agentSource = 'target/release/garage-agent.exe';
if (!existsSync(agentSource)) throw new Error('Build garage-agent before publishing');
copyFileSync(agentSource, 'downloads/project-garage-agent.exe');
const agentBytes = readFileSync('downloads/project-garage-agent.exe');
tools.agent_bridge = {
  path: 'downloads/project-garage-agent.exe',
  sha256: createHash('sha256').update(agentBytes).digest('hex'),
  size: agentBytes.length,
};
for (const [source, target] of [
  ['skills/project-garage-agent/SKILL.md', 'project-garage-agent-skill.md'],
  ['skills/project-garage-agent/references/actions.md', 'project-garage-agent-actions.md'],
  ['skills/project-garage-agent/scripts/bootstrap.ps1', 'project-garage-agent-bootstrap.ps1'],
])
  copyFileSync(source, 'downloads/' + target);
writeFileSync(
  'downloads/latest.json',
  JSON.stringify(
    {
      version,
      notes:
        'Updates, installers, and agent downloads now use the public dustindwayne/projectgarage repository. No GitHub account or token is needed.',
      platforms,
      tools,
    },
    null,
    2,
  ) + '\n',
);
console.log(
  `Prepared update channel ${version}. Commit and push downloads/ with its matching app version to main.`,
);
