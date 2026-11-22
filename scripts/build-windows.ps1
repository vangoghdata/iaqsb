# Compila IAQSB en Windows y genera el instalador (.exe con NSIS, con el logo de la app y en español, inglés o portugués).
# Requisitos: Rust (rustup, toolchain MSVC), Node.js LTS, Microsoft C++ Build Tools, CMake y LLVM.
# El script revisa cada uno y dice cómo instalar lo que falte.
# Uso, desde la carpeta raíz del proyecto:
#   powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1            (solo el instalador)
#   powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1 -Pruebas   (pruebas y luego el instalador)
#
# La primera vez tarda varios minutos: se compila HiGHS (C++) y las dependencias.
# Las siguientes reutilizan lo compilado (carpeta target\, compartida por la CLI,
# el servidor MCP y la aplicación) y solo recompilan lo que cambió.

param([switch]$Pruebas)

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

# Comprobar requisitos antes de empezar
$faltan = @()
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $faltan += "Rust:            winget install Rustlang.Rustup"
}
if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    $faltan += "Node.js LTS:     winget install OpenJS.NodeJS.LTS"
}
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$tieneMsvc = (Test-Path $vswhere) -and (& $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath)
if (-not $tieneMsvc) {
    $faltan += "C++ Build Tools: winget install Microsoft.VisualStudio.2022.BuildTools --override `"--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended`""
}
# HiGHS (el optimizador lineal) se compila con CMake y sus enlaces con libclang (LLVM)
if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) {
    $cmakeDefecto = "$env:ProgramFiles\CMake\bin"
    if (Test-Path "$cmakeDefecto\cmake.exe") { $env:Path = "$cmakeDefecto;$env:Path" }
    else { $faltan += "CMake:           winget install Kitware.CMake" }
}
if (-not $env:LIBCLANG_PATH) {
    $llvmDefecto = "$env:ProgramFiles\LLVM\bin"
    if (Test-Path "$llvmDefecto\libclang.dll") { $env:LIBCLANG_PATH = $llvmDefecto }
    elseif (Get-Command clang -ErrorAction SilentlyContinue) { $env:LIBCLANG_PATH = Split-Path (Get-Command clang).Source }
    else { $faltan += "LLVM (libclang): winget install LLVM.LLVM" }
}
if ($faltan.Count -gt 0) {
    Write-Host "Faltan programas para compilar IAQSB. Instálalos con:" -ForegroundColor Yellow
    $faltan | ForEach-Object { Write-Host "  $_" }
    Write-Host ""
    Write-Host "Después cierra esta ventana, abre una nueva de PowerShell y vuelve a ejecutar el script." -ForegroundColor Yellow
    exit 1
}

if ($Pruebas) {
    Write-Host "1/4  Pruebas del núcleo" -ForegroundColor Cyan
    # perfil «pruebas»: optimizado y sin LTO (enlaza mucho más rápido que --release)
    cargo test --profile pruebas
    if ($LASTEXITCODE -ne 0) { throw "Las pruebas fallaron" }
} else {
    Write-Host "1/4  Pruebas omitidas (usa -Pruebas para ejecutarlas)" -ForegroundColor DarkGray
}

Write-Host "2/4  Línea de comandos y servidor MCP" -ForegroundColor Cyan
cargo build --release -p iaqsb -p iaqsb-mcp
if ($LASTEXITCODE -ne 0) { throw "Falló la compilación de la CLI o del servidor MCP" }

Write-Host "3/4  Copiando binarios para el instalador" -ForegroundColor Cyan
$triple = (rustc --print host-tuple).Trim()
$bin = "apps\desktop\src-tauri\binaries"
New-Item -ItemType Directory -Force $bin | Out-Null
Copy-Item "target\release\iaqsb-mcp.exe" "$bin\iaqsb-mcp-$triple.exe" -Force
Copy-Item "target\release\iaqsb.exe" "$bin\iaqsb-$triple.exe" -Force

Write-Host "4/4  Aplicación de escritorio e instalador" -ForegroundColor Cyan
Push-Location "apps\desktop"
npm install
npx tauri build
if ($LASTEXITCODE -ne 0) { Pop-Location; throw "Falló la compilación de la aplicación" }
Pop-Location

Write-Host ""
Write-Host "Listo. Instalador en target\release\bundle\nsis\" -ForegroundColor Green
Get-ChildItem "target\release\bundle\nsis" -Filter *.exe | ForEach-Object { "  " + $_.FullName }
