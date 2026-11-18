# Сборка выпуска с подписью обновлений.
# Результат — в release\: установщик, его подпись и latest.json для автообновления.
# Эти три файла загружаются в GitHub Release с тегом v<версия>.
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root

$key = "$env:USERPROFILE\.tauri\claude-island.key"
if (-not (Test-Path $key)) { throw "Нет ключа подписи: $key" }
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $key -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ''

$conf = Get-Content 'src-tauri\tauri.conf.json' -Raw | ConvertFrom-Json
$ver = $conf.version
$repo = 'https://github.com/Iqbolovich/claude-island'

npm.cmd run tauri build
if ($LASTEXITCODE -ne 0) { throw 'Сборка не удалась' }

$src = "src-tauri\target\release\bundle\nsis\Claude Island_${ver}_x64-setup.exe"
$name = "ClaudeIsland_${ver}_x64-setup.exe"   # без пробелов: GitHub иначе переименует файл
Copy-Item $src "release\$name" -Force
Copy-Item "$src.sig" "release\$name.sig" -Force

$latest = [ordered]@{
  version   = $ver
  notes     = "Claude Island $ver"
  pub_date  = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
  platforms = @{ 'windows-x86_64' = @{
    signature = (Get-Content "$src.sig" -Raw).Trim()
    url       = "$repo/releases/download/v$ver/$name"
  } }
}
# без BOM: updater разбирает JSON строго и на BOM спотыкается
[IO.File]::WriteAllText("$root\release\latest.json", ($latest | ConvertTo-Json -Depth 5), (New-Object Text.UTF8Encoding $false))
Write-Host "Готово: release\$name, release\$name.sig, release\latest.json (v$ver)"
