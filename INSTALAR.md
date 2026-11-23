# Instalar IAQSB

Descarga el instalador de tu sistema en la [página de versiones (*Releases*)](https://github.com/vangoghdata/iaqsb/releases) del repositorio. No hace falta instalar nada más: el motor de optimización (HiGHS), la línea de comandos `iaqsb` y el servidor MCP `iaqsb-mcp` vienen incluidos.

| Sistema | Archivo |
| --- | --- |
| Windows 10 u 11 | `IAQSB_0.2.2_x64-setup.exe` |
| macOS con chip Apple (M1, M2, M3, M4…) | `IAQSB_0.2.2_aarch64.dmg` |
| macOS con procesador Intel | `IAQSB_0.2.2_x64.dmg` |
| Ubuntu, Debian, Linux Mint, Pop!_OS | `IAQSB_0.2.2_amd64.deb` |
| Fedora, openSUSE | `IAQSB-0.2.2-1.x86_64.rpm` |
| Cualquier otra distribución de Linux | `IAQSB_0.2.2_amd64.AppImage` |

¿Qué chip tiene mi Mac? Menú Apple  → **Acerca de esta Mac**. Si dice «Chip Apple M…», es el de chip Apple; si dice «Procesador Intel…», es el de Intel.

## Windows

1. Abre `IAQSB_0.2.2_x64-setup.exe` y elige el idioma.
2. Si aparece «Windows protegió su PC», pulsa **Más información** → **Ejecutar de todas formas**. Aparece porque el instalador todavía no tiene firma de código.
3. La instalación es solo para tu usuario, así que no pide permisos de administrador. Sirve también en las salas de cómputo.

## macOS

1. Abre el `.dmg` y arrastra **IAQSB** a la carpeta **Aplicaciones**.
2. La primera vez, macOS dice que no puede comprobar el desarrollador, porque la app todavía no está notarizada por Apple. Para abrirla:
   - **macOS 15 (Sequoia) o más reciente:** intenta abrir IAQSB y pulsa **Aceptar**. Ve a **Ajustes del Sistema → Privacidad y seguridad**, baja hasta «IAQSB se bloqueó…» y pulsa **Abrir igualmente**. Confirma con tu contraseña.
   - **macOS 14 o anterior:** clic derecho (o Control + clic) sobre IAQSB en Aplicaciones → **Abrir** → **Abrir**.
3. Solo hay que hacerlo una vez. Después abre con doble clic.

Si macOS dice que la app «está dañada», ejecuta esto en la Terminal y vuelve a abrirla:

```sh
xattr -dr com.apple.quarantine /Applications/IAQSB.app
```

## Linux

- **Ubuntu, Debian y derivados:** `sudo apt install ./IAQSB_0.2.2_amd64.deb`
- **Fedora y openSUSE:** `sudo dnf install ./IAQSB-0.2.2-1.x86_64.rpm` (o `sudo zypper install …`)
- **AppImage (cualquier distribución):** `chmod +x IAQSB_0.2.2_amd64.AppImage` y ábrelo con doble clic. Si no abre, instala `libfuse2` (en Ubuntu 24.04: `sudo apt install libfuse2t64`).

## Conectar con Claude

En la app pulsa **Conectar con Claude**: muestra el bloque exacto para `claude_desktop_config.json` con la ruta de tu instalación. El archivo está en:

| Sistema | Ruta |
| --- | --- |
| Windows | `%APPDATA%\Claude\claude_desktop_config.json` |
| macOS | `~/Library/Application Support/Claude/claude_desktop_config.json` |
| Linux | `~/.config/Claude/claude_desktop_config.json` |

Con la AppImage, IAQSB copia `iaqsb-mcp` a `~/.local/share/iaqsb/bin/`, porque la AppImage corre desde una carpeta temporal que cambia en cada arranque.

## Compilar desde el código

Las instrucciones están en [LEEME.md](LEEME.md#compilar-en-windows): `scripts\build-windows.ps1` en Windows y `scripts/build.sh` en macOS y Linux. Los instaladores de los tres sistemas también se generan solos en GitHub Actions (`.github/workflows/instaladores.yml`) al publicar una etiqueta `v*`.
