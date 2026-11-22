#!/usr/bin/env python3
"""Genera las imágenes del instalador NSIS a partir del ícono de la app.

    python3 tools/instalador_imagenes.py

- installer/sidebar.bmp (164 × 314): página de bienvenida y de cierre.
- installer/header.bmp  (150 × 57): cabecera de las demás páginas.
NSIS pide BMP de 24 bits; se dibujan al doble y se reducen para suavizar.
"""
import os
from PIL import Image, ImageDraw, ImageFont

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ICONO = os.path.join(RAIZ, "apps", "desktop", "src-tauri", "icons", "icon.png")
SALIDA = os.path.join(RAIZ, "apps", "desktop", "src-tauri", "installer")
FUENTES = os.path.join(RAIZ, "crates", "iaqsb-render", "fuentes")
AZUL = (43, 85, 230)
AZUL_OSCURO = (24, 52, 170)


def fuente(peso, tam):
    return ImageFont.truetype(os.path.join(FUENTES, f"Inter-{peso}.otf"), tam)


def centrado(d, y, texto, f, color, ancho):
    w = d.textlength(texto, font=f)
    d.text(((ancho - w) / 2, y), texto, font=f, fill=color)


def lateral():
    k = 3
    W, H = 164 * k, 314 * k
    im = Image.new("RGB", (W, H), AZUL)
    # degradado vertical suave
    d = ImageDraw.Draw(im)
    for y in range(H):
        t = y / H
        c = tuple(int(AZUL[i] * (1 - t) + AZUL_OSCURO[i] * t) for i in range(3))
        d.line([(0, y), (W, y)], fill=c)
    ic = Image.open(ICONO).convert("RGBA").resize((104 * k, 104 * k), Image.LANCZOS)
    # el ícono lleva su propio fondo azul redondeado: un borde blanco sutil lo separa
    marco = Image.new("RGBA", (112 * k, 112 * k), (0, 0, 0, 0))
    ImageDraw.Draw(marco).rounded_rectangle([0, 0, 112 * k - 1, 112 * k - 1], radius=26 * k, fill=(255, 255, 255, 70))
    im.paste(marco, ((W - 112 * k) // 2, 54 * k), marco)
    im.paste(ic, ((W - 104 * k) // 2, 58 * k), ic)
    d = ImageDraw.Draw(im)
    centrado(d, 184 * k, "IAQSB", fuente("SemiBold", 30 * k), (255, 255, 255), W)
    centrado(d, 222 * k, "Investigación de operaciones", fuente("Regular", 10 * k), (225, 232, 255), W)
    centrado(d, 236 * k, "Operations research", fuente("Regular", 10 * k), (225, 232, 255), W)
    centrado(d, 286 * k, "Hecho por IA Nativa", fuente("Regular", 9 * k), (200, 212, 250), W)
    return im.resize((164, 314), Image.LANCZOS)


def cabecera():
    k = 3
    W, H = 150 * k, 57 * k
    im = Image.new("RGB", (W, H), (255, 255, 255))
    ic = Image.open(ICONO).convert("RGBA").resize((40 * k, 40 * k), Image.LANCZOS)
    im.paste(ic, (W - 48 * k, (H - 40 * k) // 2), ic)
    d = ImageDraw.Draw(im)
    f = fuente("SemiBold", 17 * k)
    w = d.textlength("IAQSB", font=f)
    d.text((W - 56 * k - w, 13 * k), "IAQSB", font=f, fill=(29, 29, 31))
    f2 = fuente("Regular", 7 * k)
    w2 = d.textlength("IA Nativa", font=f2)
    d.text((W - 56 * k - w2, 35 * k), "IA Nativa", font=f2, fill=(110, 110, 115))
    return im.resize((150, 57), Image.LANCZOS)


def main():
    os.makedirs(SALIDA, exist_ok=True)
    lateral().save(os.path.join(SALIDA, "sidebar.bmp"), "BMP")
    cabecera().save(os.path.join(SALIDA, "header.bmp"), "BMP")
    print("installer/sidebar.bmp y installer/header.bmp listos")


if __name__ == "__main__":
    main()
