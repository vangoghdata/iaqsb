#!/usr/bin/env python3
"""Genera docs/FUENTES.md y docs/iaqsb.bib a partir de la bibliografía central.

Uso:  python3 tools/generar_fuentes.py [ruta/al/binario/iaqsb]

Los datos salen de crates/iaqsb-core/fuentes/*.json a través de la CLI, así que
los documentos siempre coinciden con lo que muestra la aplicación.
"""
import json
import os
import subprocess
import sys

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CLI = sys.argv[1] if len(sys.argv) > 1 else os.path.join(RAIZ, "target", "release", "iaqsb")


def llamar(op, args=None):
    p = subprocess.run([CLI, "--json", "llamar", op], input=json.dumps(args or {}), capture_output=True, text=True, encoding="utf-8", check=True)
    r = json.loads(p.stdout)
    if not r.get("ok"):
        raise SystemExit(r)
    return r["result"]


def main():
    bib = llamar("bibliography")
    ejemplos = llamar("examples")
    os.makedirs(os.path.join(RAIZ, "docs"), exist_ok=True)

    # newline="\n": los mismos finales de línea (LF) en Windows que en Linux y macOS
    with open(os.path.join(RAIZ, "docs", "iaqsb.bib"), "w", encoding="utf-8", newline="\n") as f:
        f.write("% Bibliografía de IAQSB (generada por tools/generar_fuentes.py)\n\n")
        f.write("\n".join(r["bibtex"] for r in bib))

    L = []
    L.append("# Fuentes de IAQSB\n")
    L.append("Todo método, fórmula, paso y ejemplo de IAQSB dice de dónde sale. Este documento se genera con "
             "`python3 tools/generar_fuentes.py` a partir de `crates/iaqsb-core/fuentes/bibliografia.json` y "
             "`crates/iaqsb-core/fuentes/ejemplos.json` y `crates/iaqsb-core/fuentes/metodos.json`; la aplicación muestra lo mismo en la pestaña **Fuentes** de cada informe, "
             "la CLI con `iaqsb fuentes` (y `--bibtex`) y el servidor MCP con `get_bibliography`.\n")
    L.append("- **Estilo:** APA 7.ª edición, en español.")
    L.append("- **Verificada:** los datos se comprobaron en el DOI, el editor o un catálogo de biblioteca.")
    L.append("- **Por verificar:** falta confirmar algo (se dice qué). Nada se inventa: si no se conoce una página o una edición, no se escribe.")
    L.append("- **Material de clase:** las diapositivas y los talleres del curso Taller de Producción y Logística de la carrera de "
             "Ingeniería Industrial de la Universidad Nacional de Colombia se citan por su profesor: Sarache (2026a, b, c…) en logística "
             "y Arango Marín (2020, 2021) en programa maestro y MRP II, aunque la diapositiva a su vez cite un libro; si la diapositiva "
             "nombra la fuente original (Bronson, Ballou, Taha), se cita también. El año de las diapositivas de Arango Marín sale de "
             "los metadatos de los archivos y queda por verificar.")
    L.append("- **Métodos:** cada informe cita los métodos que usó. Los módulos de logística lo hacen desde su código y los demás "
             "con las reglas de `crates/iaqsb-core/fuentes/metodos.json` (resumidas abajo).\n")

    pend = [r for r in bib if not r["verified"]]
    L.append(f"## Referencias ({len(bib)}, {len(pend)} por verificar)\n")
    for r in bib:
        estado = "" if r["verified"] else f" **[Por verificar: {r['pending']}]**"
        nota = f" — _{r['note']}_" if r.get("note") else ""
        L.append(f"- {r['apa']}{estado}{nota}  \n  `{r['id']}` · {r.get('topic', '')}")
    L.append("")

    L.append("## Origen de los datos de cada ejemplo\n")
    L.append("| Módulo | Ejemplo | Origen | Fuente | Estado |")
    L.append("|---|---|---|---|---|")
    sin = []
    for e in ejemplos:
        s = e.get("source")
        if not s:
            sin.append(f"{e['module']}/{e['name']}")
            continue
        est = "Verificada" if s["verified"] else f"Por verificar: {s.get('pending', '')}"
        L.append(f"| {e['module']} | {e['model'].get('name') or e['name']} | {s['origin_label']} | {s['summary']} | {est} |")
    L.append("")
    if sin:
        L.append(f"### Ejemplos que aún no tienen fuente registrada ({len(sin)})\n")
        L.append("Se irán documentando módulo por módulo: " + ", ".join(f"`{x}`" for x in sin) + ".\n")

    reglas = json.load(open(os.path.join(RAIZ, "crates", "iaqsb-core", "fuentes", "metodos.json"), encoding="utf-8"))["reglas"]
    cita = {r["id"]: r["cite"] for r in bib}
    L.append("## Métodos que cita cada módulo\n")
    L.append("| Módulo | Cuándo | Fuente | Para |")
    L.append("|---|---|---|---|")
    for g in reglas:
        cuando = "; ".join(f"{k} = {', '.join(v)}" for k, v in g["si"].items()) or "siempre"
        for x in g["refs"]:
            L.append(f"| {g['modulo']} | {cuando} | {cita.get(x['id'], x['id'])} | {x['para']} |")
    L.append("")

    with open(os.path.join(RAIZ, "docs", "FUENTES.md"), "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(L) + "\n")
    print(f"docs/FUENTES.md y docs/iaqsb.bib: {len(bib)} referencias, {len(ejemplos) - len(sin)} ejemplos con fuente, {len(sin)} sin fuente")


if __name__ == "__main__":
    main()
