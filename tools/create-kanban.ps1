<#
  Crée sur le board Kanban Hermes les tâches de Puce8_Galaxian (une tâche = un fichier docs/tasks/Txxx.md).
  Usage (PowerShell, depuis le dépôt) :
      .\tools\create-kanban.ps1 -DryRun        # affiche sans rien créer
      .\tools\create-kanban.ps1                # crée les tâches (idempotent : relançable sans doublon)
  Chaque tâche dépend de la précédente (--parent) : Hermes ne la passe en « ready » que quand le parent est « done ».
#>
param(
  [string]$Workspace = 'dir:H:\__Emulator__\Puce8_Galaxian',
  [string]$Assignee  = 'rust-dev',
  [string]$Index     = (Join-Path $PSScriptRoot '..\tasks\index.csv'),
  [string]$From = '',
  [switch]$DryRun
)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8
$ids = @{}
$rows = Import-Csv -Path $Index -Encoding UTF8
$started = ($From -eq '')
$count = 0
foreach ($r in $rows) {
  if (-not $started) { if ($r.key -eq $From) { $started = $true } else { continue } }
  $count++
  $body = "Exécute À LA LETTRE le fichier $($r.file) du workspace (lis d'abord docs/tasks/_REGLES.md). Tu n'as aucune décision à prendre : recopie, lance les commandes, compare aux sorties attendues. Si une sortie diffère : bloque la tâche, n'improvise pas."
  $a = @('kanban','create',"$($r.key) $($r.title)",'--body',$body,'--assignee',$Assignee,'--workspace',$Workspace,
         '--priority',$r.priority,'--idempotency-key',"puce8-galaxian-$($r.key)",'--json')
  foreach ($p in ($r.parents -split ';' | Where-Object { $_ })) { if ($ids.ContainsKey($p)) { $a += @('--parent', $ids[$p]) } }
  if ($DryRun) { $ids[$r.key] = "DRY-$($r.key)"; Write-Host "[dry] $($r.key)  $($r.title)  <- $($r.parents)"; continue }
  $out = (& hermes @a) -join "`n"
  $id = $null
  try { $id = ($out | ConvertFrom-Json).id } catch {}
  if (-not $id -and $out -match '(t_[A-Za-z0-9]+)') { $id = $Matches[1] }
  if (-not $id) { throw "ID introuvable pour $($r.key) : $out" }
  $ids[$r.key] = $id
  Write-Host "[$($r.key)] $id  $($r.title)"
}
Write-Host "Terminé : $count tâches."
