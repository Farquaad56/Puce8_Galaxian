<#
  Génère les golden logs avec MAME 0.287 puis les compare à l'émulateur.
  Usage : .\tools\golden\run-golden.ps1 -Frames 700 [-Inputs tools\inputs\anchor_play.txt] [-Mode cpu|video|all]
#>
param(
  [int]$Frames = 700,
  [string]$Inputs = 'tools\inputs\anchor_play.txt',
  [ValidateSet('cpu','video','all')][string]$Mode = 'all',
  [string]$Mame = 'X:\Mame_0287\mame.exe',
  [string]$RomPath = 'X:\Mame_0287\roms',
  [string]$Cli = '.\target\release\puce8-galaxian.exe'
)
$ErrorActionPreference = 'Stop'
$zip = Join-Path $RomPath 'galaxian.zip'
New-Item -ItemType Directory -Force golden | Out-Null
$env:GOLDEN_FRAMES = "$Frames"
$env:GOLDEN_INPUTS = (Resolve-Path $Inputs).Path
$mameDir = Split-Path $Mame

if ($Mode -in 'cpu','all') {
  Remove-Item golden\mame_raw.log -ErrorAction SilentlyContinue
  Remove-Item Env:\GOLDEN_VIDEO -ErrorAction SilentlyContinue
  New-Item -ItemType Directory -Force (Join-Path $mameDir 'golden') | Out-Null
  Push-Location $mameDir
  & $Mame galaxian -rompath $RomPath -debug -debugscript (Join-Path $PSScriptRoot '..\mame\trace.dbg') `
    -autoboot_script (Join-Path $PSScriptRoot '..\mame\golden.lua') -nothrottle -sound none -window -skip_gameinfo
  Pop-Location
  # MAME écrit golden/mame_raw.log relativement à son dossier courant : on le ramène dans le dépôt.
  $raw = Join-Path $mameDir 'golden\mame_raw.log'
  if (Test-Path $raw) { Move-Item -Force $raw golden\mame_raw.log }
  Get-Content golden\mame_raw.log | Where-Object { $_ -like 'C *' } | Set-Content golden\mame_cpu.log -Encoding ascii
  Get-Content golden\mame_raw.log | Where-Object { $_ -like 'B *' } | Set-Content golden\mame_bus.log -Encoding ascii
  # MAME n'enregistre pas l'instruction à l'adresse 0000 (la trace démarre après elle) : on écarte la première ligne C de l'émulateur.
  & $Cli trace-cpu $zip --frames $Frames --inputs $Inputs --out golden\emu_cpu_full.log
  Get-Content golden\emu_cpu_full.log | Select-Object -Skip 1 | Set-Content golden\emu_cpu.log -Encoding ascii
  & $Cli trace-bus $zip --frames $Frames --inputs $Inputs --out golden\emu_bus.log
  Write-Host 'Logs MAME et émulateur générés dans golden\ (mame_cpu.log, mame_bus.log, emu_cpu.log, emu_bus.log).'
}

if ($Mode -in 'video','all') {
  $env:GOLDEN_VIDEO = (Join-Path (Get-Location) 'golden\mame_video.raw')
  Remove-Item $env:GOLDEN_VIDEO -ErrorAction SilentlyContinue
  & $Mame galaxian -rompath $RomPath -autoboot_script (Join-Path $PSScriptRoot '..\mame\golden.lua') `
    -nothrottle -sound none -window -skip_gameinfo
  & $Cli raw-crc golden\mame_video.raw --width 768 --height 224 --format bgra --out golden\mame_video.crc
  & $Cli crc-frames $zip --frames $Frames --inputs $Inputs --out golden\emu_video.crc
  Write-Host 'CRC vidéo MAME et émulateur générés dans golden\.'
}
