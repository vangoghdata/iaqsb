from nativo import modelo, Informe

cf = float(modelo.get("costo_fijo") or 0)
p = float(modelo.get("precio") or 0)
cv = float(modelo.get("costo_variable") or 0)
cap = float(modelo.get("capacidad") or 0)
u = modelo.get("unidad") or "unidades"


def dinero(x):
    return f"{x:,.0f}".replace(",", ".") if abs(x) >= 1000 else f"{x:g}"


def calcular():
    margen = p - cv
    if margen <= 0:
        inf = Informe("No hay punto de equilibrio: el precio no supera el costo variable", estado="infeasible")
        inf.texto(f"Cada {u[:-1] if u.endswith('s') else u} se vende en {dinero(p)} y cuesta {dinero(cv)}: el margen de contribución es {dinero(margen)}. Vender más solo aumenta la pérdida. Hay que subir el precio o bajar el costo variable.", tono="bad")
        inf.publicar()
        return

    q = cf / margen
    ventas_eq = q * p
    utilidad_cap = margen * cap - cf
    seguridad = (cap - q) / cap * 100 if cap > 0 else 0

    inf = Informe(f"Punto de equilibrio: {q:.1f} {u} ({dinero(ventas_eq)} en ventas)")
    inf.metricas([
        ("Punto de equilibrio", round(q, 2), u),
        ("Ventas de equilibrio", round(ventas_eq, 2)),
        ("Margen de contribución", round(margen, 2), "por unidad"),
        ("Utilidad a la capacidad", round(utilidad_cap, 2)),
        ("Margen de seguridad", round(seguridad, 1), "%"),
    ])
    if utilidad_cap < 0:
        inf.texto(f"Con {cap:g} {u} la empresa pierde {dinero(-utilidad_cap)}: necesita vender al menos {q:,.1f} {u} para no perder.", tono="bad")
    else:
        inf.texto(f"Con {cap:g} {u} la empresa gana {dinero(utilidad_cap)}. Las ventas pueden caer {seguridad:.1f} % antes de empezar a perder.", tono="good")

    tope = max(cap, q) * 1.4
    puntos = [tope * k / 10 for k in range(11)]
    inf.grafica(
        {
            "Ingresos": [(x, p * x) for x in puntos],
            "Costo total": [(x, cf + cv * x) for x in puntos],
            "Costos fijos": [(x, cf) for x in puntos],
        },
        titulo="Ingresos y costos",
        x=u,
        y="Dinero",
    )
    filas = []
    for x in sorted({0, round(q / 2), round(q), round(cap), round(tope)}):
        filas.append([x, p * x, cf + cv * x, margen * x - cf])
    inf.tabla([u.capitalize(), "Ingresos", "Costo total", "Utilidad"], filas, titulo="Ingresos, costos y utilidad", tonos=["bad" if f[3] < 0 else "good" for f in filas])

    inf.paso("Margen de contribución", f"p − cv = {p:g} − {cv:g} = {margen:g} por unidad: lo que deja cada venta para cubrir los costos fijos.")
    inf.paso("Punto de equilibrio en unidades", f"Q* = CF / (p − cv) = {cf:g} / {margen:g} = {q:.4g} {u}")
    inf.paso("En dinero", f"Q* · p = {q:.4g} · {p:g} = {ventas_eq:.6g}")
    inf.paso("Utilidad a la capacidad", f"U = (p − cv) · Q − CF = {margen:g} · {cap:g} − {cf:g} = {utilidad_cap:.6g}")
    inf.datos(punto_equilibrio=q, ventas_equilibrio=ventas_eq, margen_contribucion=margen, utilidad_capacidad=utilidad_cap, margen_seguridad=seguridad)
    inf.publicar()


calcular()
