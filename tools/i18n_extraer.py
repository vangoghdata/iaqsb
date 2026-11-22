#!/usr/bin/env python3
"""Extrae los textos en español del núcleo (Rust y datos JSON) para traducirlos.

Uso: python3 tools/i18n_extraer.py SALIDA.json

Cada texto queda como plantilla: los marcadores de formato de Rust ({}, {x},
{:.2}...) se cambian por {0}, {1}... en orden. La traducción al inglés vive en
crates/iaqsb-core/i18n/en.json (español → inglés) y la usa iaqsb_core::i18n.
"""
import glob
import json
import os
import re
import sys

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PH = re.compile(r"\{(?!\{)[^{}]*\}")


def unescape(s):
    out, i = [], 0
    while i < len(s):
        c = s[i]
        if c == "\\" and i + 1 < len(s):
            n = s[i + 1]
            if n == "n":
                out.append("\n"); i += 2
            elif n == "t":
                out.append("\t"); i += 2
            elif n in "\"'\\":
                out.append(n); i += 2
            elif n == "u":
                m = re.match(r"\\u\{([0-9a-fA-F]+)\}", s[i:])
                out.append(chr(int(m.group(1), 16))); i += len(m.group(0))
            elif n == "\n":
                i += 2
                while i < len(s) and s[i] in " \t\n":
                    i += 1
            else:
                out.append(n); i += 2
        else:
            out.append(c); i += 1
    return "".join(out)


def plantilla(s):
    """Marcadores de Rust → {0}, {1}…; None si no hay nada que traducir."""
    if "{{" in s or "}}" in s:
        s = s.replace("{{", "\x01").replace("}}", "\x02")
    k = [0]

    def rep(m):
        r = "{%d}" % k[0]
        k[0] += 1
        return r

    t = PH.sub(rep, s).replace("\x01", "{").replace("\x02", "}")
    return t


def traducible(t):
    sin = re.sub(r"\{\d+\}", "", t)
    if not re.search(r"[A-Za-zÁÉÍÓÚáéíóúÑñ]{2,}", sin):
        return False
    core = sin.strip()
    # códigos: minúsculas sin espacios ni tildes (claves, ids, valores de opciones);
    # las plantillas con palabras sueltas («{0} en {1}») sí se traducen
    if re.fullmatch(r"[a-z0-9_.\-/:#%+=<>()\[\]]+", core) and not re.search(r"[áéíóúñ]", core):
        if not ("{" in t and re.fullmatch(r"[a-z]{2,}", core) and t.strip() != t.strip("{}0123456789")):
            return False
    if "_" in core and " " not in core:
        return False
    if re.fullmatch(r"[A-Z0-9_\-]+", core) and len(core) <= 6:  # siglas (LP, EOQ, CMAX…)
        return False
    if core.startswith("<") or re.match(r"^[ML]\s?-?\d", core):
        return False  # SVG / HTML y trazos de SVG
    return True


SALTAR = re.compile(r"(expect|panic!|unreachable!|assert(_eq|_ne)?!|debug_assert!|todo!|unimplemented!)\s*\(\s*$")
LIT = re.compile(r'(?<![A-Za-z0-9_#])"((?:[^"\\]|\\.|\\\n)*)"', re.S)


def rust():
    textos = {}
    archivos = sorted(f for f in glob.glob(os.path.join(RAIZ, "crates", "*", "src", "**", "*.rs"), recursive=True) if not f.endswith("i18n.rs"))
    archivos += sorted(glob.glob(os.path.join(RAIZ, "apps", "cli", "src", "**", "*.rs"), recursive=True))
    archivos += sorted(glob.glob(os.path.join(RAIZ, "apps", "mcp", "src", "**", "*.rs"), recursive=True))
    archivos += sorted(glob.glob(os.path.join(RAIZ, "apps", "desktop", "src-tauri", "src", "*.rs")))
    for f in archivos:
        src = open(f, encoding="utf-8").read()
        # sin los módulos de pruebas
        src = re.split(r"\n#\[cfg\(test\)\]\s*\nmod tests", src)[0]
        # sin comentarios de línea (cuidando las cadenas)
        limpio = []
        for linea in src.split("\n"):
            if linea.lstrip().startswith("//"):
                limpio.append("")
            else:
                limpio.append(linea)
        src = "\n".join(limpio)
        for m in LIT.finditer(src):
            antes = src[max(0, m.start() - 40):m.start()]
            if SALTAR.search(antes):
                continue
            if src[max(0, m.start() - 1):m.start()] == "r" or src[max(0, m.start() - 2):m.start()] == "r#":
                continue
            s = unescape(m.group(1))
            t = plantilla(s)
            if not traducible(t):
                continue
            linea = src.count("\n", 0, m.start()) + 1
            textos.setdefault(t, f"{os.path.relpath(f, RAIZ)}:{linea}")
    return textos


CAMPOS_FUENTES = {"descripcion", "nota", "pendiente", "tema", "donde", "resultado", "para"}


def datos():
    textos = {}
    for nombre in ("bibliografia.json", "ejemplos.json", "metodos.json"):
        f = os.path.join(RAIZ, "crates", "iaqsb-core", "fuentes", nombre)

        def walk(v, clave):
            if isinstance(v, dict):
                for k, x in v.items():
                    walk(x, k)
            elif isinstance(v, list):
                for x in v:
                    walk(x, clave)
            elif isinstance(v, str) and clave in CAMPOS_FUENTES and traducible(v):
                textos.setdefault(v.replace("{", "(").replace("}", ")"), f"fuentes/{nombre}:{clave}")

        walk(json.load(open(f, encoding="utf-8")), "")
    # modelos de ejemplo: todos los textos que no son códigos
    for f in sorted(glob.glob(os.path.join(RAIZ, "crates", "**", "ejemplos", "**", "*.json"), recursive=True)):
        def walk(v, clave):
            if isinstance(v, dict):
                for k, x in v.items():
                    walk(x, k)
            elif isinstance(v, list):
                for x in v:
                    walk(x, clave)
            elif isinstance(v, str) and clave not in ("problem_type", "kind", "type", "direction", "id") and traducible(v):
                textos.setdefault(v, f"{os.path.relpath(f, RAIZ)}:{clave}")

        walk(json.load(open(f, encoding="utf-8")), "")
    return textos


def main():
    out = sys.argv[1]
    t = rust()
    for k, v in datos().items():
        t.setdefault(k, v)
    json.dump([{"es": k, "donde": v} for k, v in t.items()], open(out, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    print(f"{len(t)} textos, {sum(len(k) for k in t)} caracteres")


if __name__ == "__main__":
    main()
