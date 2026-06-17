# Aftercode CLI installer for Windows (PowerShell).
#   irm https://raw.githubusercontent.com/ron3899/aftercode/main/install.ps1 | iex
# Downloads the latest prebuilt aftercode.exe — no Rust or build tools required.
$ErrorActionPreference = "Stop"

$repo  = "ron3899/aftercode"
$asset = "aftercode-x86_64-pc-windows-msvc.zip"
$dest  = Join-Path $env:LOCALAPPDATA "Programs\aftercode"
$url   = "https://github.com/$repo/releases/latest/download/$asset"

Write-Host "Downloading $url"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
$zip = Join-Path $env:TEMP $asset
Invoke-WebRequest -Uri $url -OutFile $zip
Expand-Archive -Path $zip -DestinationPath $dest -Force
Remove-Item $zip

# Add the install dir to the user PATH if it isn't there yet.
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$dest*") {
  [Environment]::SetEnvironmentVariable("Path", "$userPath;$dest", "User")
  Write-Host "Added $dest to your PATH. Restart your terminal for it to take effect."
}

Write-Host ""
Write-Host "Installed aftercode to $dest"
Write-Host "Next: start the backend with Docker, then run:  aftercode login"
