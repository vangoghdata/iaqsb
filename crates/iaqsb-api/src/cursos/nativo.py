"""Ayuda para los temas de cursos de Nativo (.ncourse) que calculan en Python.

El programa del tema lee los datos de la pantalla con `modelo` (y las opciones con `opciones`),
arma un Informe y lo publica:

    from nativo import modelo, Informe
    inf = Informe("Punto de equilibrio: 1250 unidades")
    inf.metricas([("Punto de equilibrio", 1250, "unidades")])
    inf.tabla(["Unidades", "Ingreso", "Costo"], [[0, 0, 5000], [1250, 12500, 12500]], titulo="Tabla")
    inf.grafica({"Ingreso": [(0, 0), (2500, 25000)], "Costo total": [(0, 5000), (2500, 20000)]}, titulo="Equilibrio", x="Unidades", y="Pesos")
    inf.paso("Fórmula", "Q* = CF / (p − cv) = 5000 / (10 − 6) = 1250")
    inf.publicar()

Todo lo que se imprima con print() antes de publicar() se muestra como avisos.
"""
import json as _json
import math as _math

with open("entrada.json", encoding="utf-8") as _f:
    _E = _json.load(_f)

#: Datos que el usuario escribió en la pantalla (el modelo del tema).
modelo = _E.get("model") or {}
#: Opciones del tema.
opciones = _E.get("options") or {}
#: Idioma de la interfaz: "es", "en" o "pt".
idioma = _E.get("lang") or "es"
#: En un tema basado en un módulo de Nativo (tipo «modulo» con «ajuste»): el informe que dio el
#: módulo, para cambiarlo o completarlo con Informe.desde(base).
base = _E.get("base")

_TONOS = {"normal", "accent", "highlight", "good", "bad", "muted", "total"}


def _num(v):
    if isinstance(v, bool):
        return v
    if isinstance(v, (int, float)):
        return None if isinstance(v, float) and not _math.isfinite(v) else v
    try:
        import numpy as _np  # noqa: F401

        if hasattr(v, "item"):
            return _num(v.item())
    except Exception:
        pass
    return v if v is None or isinstance(v, str) else str(v)


class Informe:
    """El informe que se ve en la pantalla de resultados (como los de los módulos de Nativo)."""

    def __init__(self, titular, estado="solved"):
        self.titular = str(titular)
        self.estado = estado if estado in ("optimal", "feasible", "infeasible", "unbounded", "solved") else "solved"
        self._secciones = {}
        self._orden = []
        self._pasos = []
        self._datos = {}
        self._extra = {}

    @classmethod
    def desde(cls, informe):
        """Un Informe que empieza con todo lo del informe de un módulo (secciones, paso a paso y
        datos): se le agregan tablas, métricas o pasos, o se cambia el titular, y se publica."""
        informe = informe or {}
        inf = cls(informe.get("headline", ""), informe.get("status", "solved"))
        for s in informe.get("sections") or []:
            if s.get("id") == "pasos":
                for b in s.get("blocks") or []:
                    if b.get("type") == "steps":
                        inf._pasos.extend(b.get("steps") or [])
                continue
            inf._secciones[s["id"]] = {"id": s["id"], "title": s.get("title", s["id"]), "blocks": list(s.get("blocks") or [])}
            inf._orden.append(s["id"])
        inf._datos.update(informe.get("data") or {})
        inf._extra = {k: v for k, v in informe.items() if k not in ("sections", "headline", "status", "data", "module", "problem")}
        return inf

    def _poner(self, seccion, bloque, titulo_seccion=None):
        if seccion not in self._secciones:
            self._secciones[seccion] = {"id": seccion, "title": titulo_seccion or {"solucion": "Solución", "detalle": "Detalle"}.get(seccion, seccion.capitalize()), "blocks": []}
            self._orden.append(seccion)
        self._secciones[seccion]["blocks"].append(bloque)
        return bloque

    def metricas(self, items, titulo=None, seccion="solucion"):
        """items: lista de (etiqueta, valor) o (etiqueta, valor, unidad). La primera se destaca."""
        lista = []
        for k, it in enumerate(items):
            etiqueta, valor = it[0], it[1]
            m = {"label": str(etiqueta), "value": _num(valor)}
            if len(it) > 2 and it[2]:
                m["unit"] = str(it[2])
            if k == 0:
                m["emphasis"] = True
            lista.append(m)
        return self._poner(seccion, {"type": "metrics", **({"title": titulo} if titulo else {}), "items": lista})

    def tabla(self, columnas, filas, titulo=None, nota=None, tonos=None, seccion="solucion"):
        """columnas: nombres; filas: listas de valores. tonos: lista con el tono de cada fila (o None)."""
        cols = [{"label": str(c)} for c in columnas]
        for k, c in enumerate(cols):
            if any(isinstance(_num(f[k]), (int, float)) for f in filas if k < len(f)):
                c["align"] = "right"
        rows = []
        for k, f in enumerate(filas):
            r = {"cells": [_num(x) for x in f]}
            if tonos and k < len(tonos) and tonos[k] in _TONOS:
                r["tone"] = tonos[k]
            rows.append(r)
        b = {"type": "table", "columns": cols, "rows": rows}
        if titulo:
            b["title"] = titulo
        if nota:
            b["note"] = nota
        return self._poner(seccion, b)

    def texto(self, cuerpo, titulo=None, tono=None, seccion="solucion"):
        b = {"type": "text", "body": str(cuerpo)}
        if titulo:
            b["title"] = titulo
        if tono in _TONOS:
            b["tone"] = tono
        return self._poner(seccion, b)

    def grafica(self, series, titulo=None, x="", y="", tipo="line", categorias=None, lineas_h=None, seccion="solucion"):
        """series: {nombre: [(x, y), ...]} o {nombre: [y1, y2, ...]}; tipo: line, bar, scatter, step o area."""
        lista = []
        for nombre, pts in series.items():
            pts = list(pts)
            if pts and not isinstance(pts[0], (list, tuple)):
                pts = list(enumerate(pts))
            lista.append({"name": str(nombre), "kind": tipo, "points": [[_num(a), _num(b)] for a, b in pts]})
        b = {"type": "chart", "x_label": str(x), "y_label": str(y), "series": lista}
        if titulo:
            b["title"] = titulo
        if categorias:
            b["categories"] = [str(c) for c in categorias]
        if lineas_h:
            b["hlines"] = [{"value": _num(v), "label": str(e)} for e, v in lineas_h]
        return self._poner(seccion, b)

    def paso(self, titulo, descripcion="", bloques=None):
        """Un paso del «Paso a paso» (con fórmulas en el texto, por ejemplo Q* = √(2DS/H))."""
        self._pasos.append({"title": str(titulo), "description": str(descripcion), "blocks": bloques or []})

    def datos(self, **valores):
        """Resultados estructurados (para la IA y para exportar)."""
        self._datos.update({k: _num(v) if not isinstance(v, (list, dict)) else v for k, v in valores.items()})

    def como_dict(self):
        secciones = [self._secciones[s] for s in self._orden]
        if self._pasos:
            secciones.append({"id": "pasos", "title": "Paso a paso", "blocks": [{"type": "steps", "steps": self._pasos}]})
        return {**self._extra, "module": "", "problem": str(modelo.get("name", "")), "status": self.estado, "headline": self.titular, "sections": secciones, "data": self._datos}

    def publicar(self):
        print("@@NATIVO_INFORME@@" + _json.dumps(self.como_dict(), ensure_ascii=False, allow_nan=False, default=str))
