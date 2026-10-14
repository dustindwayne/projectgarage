param([string]$OutputDirectory = (Join-Path (Get-Location).Path '.project-garage-agent'))
$ErrorActionPreference='Stop'
$taskReference=Invoke-RestMethod -Uri 'https://api.github.com/repos/dustindwayne/projectgarage/git/ref/heads/main'
$taskCommit=$taskReference.object.sha
if($taskCommit -notmatch '^[0-9a-f]{40}$'){throw 'Could not resolve the published bridge version'}
$taskBase="https://raw.githubusercontent.com/dustindwayne/projectgarage/$taskCommit/downloads"
$taskManifest=Invoke-RestMethod -Uri "$taskBase/latest.json"
$taskPackage=$taskManifest.tools.agent_bridge
if($taskPackage.path -ne 'downloads/project-garage-agent.exe' -or $taskPackage.sha256 -notmatch '^[0-9a-fA-F]{64}$' -or $taskPackage.size -lt 1 -or $taskPackage.size -gt 50000000){throw 'Invalid bridge manifest'}
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$taskTarget=Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) 'garage-agent.exe'
$taskPending=Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) ('download-'+[guid]::NewGuid().ToString()+'.part')
try {
 Invoke-WebRequest -Uri "$taskBase/project-garage-agent.exe" -OutFile $taskPending
 if((Get-Item -LiteralPath $taskPending).Length -ne $taskPackage.size -or (Get-FileHash -LiteralPath $taskPending -Algorithm SHA256).Hash.ToLowerInvariant() -ne $taskPackage.sha256){throw 'Bridge download failed verification'}
 Move-Item -LiteralPath $taskPending -Destination $taskTarget -Force
 Write-Output $taskTarget
} finally {if(Test-Path -LiteralPath $taskPending){Remove-Item -LiteralPath $taskPending}}
