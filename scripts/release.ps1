# Сборка выпуска с подписью обновлений — одной командой.
#
#   scripts\release.ps1                                  собрать текущую версию
#   scripts\release.ps1 -Version 0.2.9                   поднять версию и собрать
#   scripts\release.ps1 -Version 0.2.9 -Publish -Notes "Что нового"
#                                                        поднять, собрать и выпустить на GitHub
#
# Результат — в release\: установщик, его подпись и latest.json для автообновления.
# -Publish загружает эти три файла в GitHub Release v<версия> через GitHub CLI (gh),
# нужен один раз выполненный «gh auth login».
param(
  [string]$Version = '',
  [switch]$Publish,
  [string]$Notes = ''
)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
$utf8 = New-Object Text.UTF8Encoding $false
$repoSlug = 'Iqbolovich/claude-island'
$repo = "https://github.com/$repoSlug"

if ($Publish -and -not (Get-Command gh -ErrorAction SilentlyContinue)) {
  throw 'Нет GitHub CLI (gh). Установи: winget install GitHub.cli, затем выполни gh auth login'
}

# версия во всех трёх местах сразу (без BOM — иначе сборка Tauri не читает конфиг)
if ($Version) {
  $conf0 = Get-Content 'src-tauri\tauri.conf.json' -Raw | ConvertFrom-Json
  $old = $conf0.version
  foreach ($f in 'src-tauri\tauri.conf.json', 'package.json') {
    $t = [IO.File]::ReadAllText("$root\$f").Replace("`"version`": `"$old`"", "`"version`": `"$Version`"")
    [IO.File]::WriteAllText("$root\$f", $t, $utf8)
  }
  $t = [IO.File]::ReadAllText("$root\src-tauri\Cargo.toml")
  $t = ([regex]'(?m)^version = "[^"]+"').Replace($t, "version = `"$Version`"", 1)
  [IO.File]::WriteAllText("$root\src-tauri\Cargo.toml", $t, $utf8)
  Write-Host "Версия: $old -> $Version"
}

$key = "$env:USERPROFILE\.tauri\claude-island.key"
if (-not (Test-Path $key)) { throw "Нет ключа подписи: $key" }
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $key -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ''

$conf = Get-Content 'src-tauri\tauri.conf.json' -Raw | ConvertFrom-Json
$ver = $conf.version

$ErrorActionPreference = 'Continue'   # сборка пишет прогресс в stderr — это не ошибка
npm.cmd run tauri build
if ($LASTEXITCODE -ne 0) { throw 'Сборка не удалась' }
$ErrorActionPreference = 'Stop'

New-Item -ItemType Directory -Force "$root\release" | Out-Null
Remove-Item "$root\release\*.exe", "$root\release\*.sig", "$root\release\latest.json" -ErrorAction SilentlyContinue
$src = "src-tauri\target\release\bundle\nsis\Claude Island_${ver}_x64-setup.exe"
$name = "ClaudeIsland_${ver}_x64-setup.exe"   # без пробелов: GitHub иначе переименует файл
Copy-Item $src "release\$name" -Force
Copy-Item "$src.sig" "release\$name.sig" -Force

$latest = [ordered]@{
  version   = $ver
  notes     = if ($Notes) { $Notes } else { "Claude Island $ver" }
  pub_date  = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
  platforms = @{ 'windows-x86_64' = @{
    signature = (Get-Content "$src.sig" -Raw).Trim()
    url       = "$repo/releases/download/v$ver/$name"
  } }
}
# без BOM: updater разбирает JSON строго и на BOM спотыкается
[IO.File]::WriteAllText("$root\release\latest.json", ($latest | ConvertTo-Json -Depth 5), $utf8)
Write-Host "Собрано: release\$name, release\$name.sig, release\latest.json (v$ver)"

if ($Publish) {
  $files = @("release\$name", "release\$name.sig", "release\latest.json")
  $body = if ($Notes) { $Notes } else { "Claude Island $ver" }
  gh release create "v$ver" @files --repo $repoSlug --title "Claude Island $ver" --notes $body --target main --latest
  if ($LASTEXITCODE -ne 0) { throw 'Выпуск на GitHub не удался' }
  Write-Host "Выпущено: $repo/releases/tag/v$ver"
}
