# Puente de IAQSB con statsforecast (Nixtla): lee los trabajos en JSON por la
# entrada estándar y devuelve los ajustes en JSON. Lo escribe IAQSB en una
# carpeta temporal y lo ejecuta solo si statsforecast está instalado.
import json
import math
import sys

import numpy as np


def lista(v):
    out = []
    for x in np.asarray(v, dtype=float).ravel().tolist():
        out.append(x if math.isfinite(x) else None)
    return out


def boxcox(y, lam):
    if lam is None:
        return y
    return np.log(y) if lam == 0 else (np.sign(y) * np.abs(y) ** lam - 1) / lam


def inv_boxcox(y, lam):
    y = np.asarray(y, dtype=float)
    if lam is None:
        return y
    if lam == 0:
        return np.exp(y)
    z = lam * y + 1
    return np.sign(z) * np.abs(z) ** (1 / lam)


def modelo(fam, extra, m):
    from statsforecast import models as M

    s = max(1, int(round(m)))
    if fam == "arima":
        o = extra.get("orden")
        if o is None:
            return M.AutoARIMA(season_length=s)
        return M.ARIMA(order=(o[0], o[1], o[2]), seasonal_order=(o[3], o[4], o[5]), season_length=s, include_drift=bool(extra.get("deriva")))
    if fam == "ets":
        spec = extra.get("spec")
        if spec is None:
            return M.AutoETS(season_length=s)
        u = spec.upper()
        damped = "AD" in u or "MD" in u
        return M.AutoETS(season_length=s, model=u.replace("D", ""), damped=damped)
    if fam == "tbats":
        per = extra.get("periodos") or [m]
        # statsforecast pide enteros cuando el periodo es entero (12, no 12.0)
        return M.AutoTBATS(season_length=[int(p) if float(p).is_integer() else float(p) for p in per])
    if fam == "theta":
        return M.AutoTheta(season_length=s)
    if fam == "croston":
        return M.CrostonClassic()
    if fam == "ingenuo":
        return M.Naive()
    if fam == "ingenuo_estacional":
        return M.SeasonalNaive(season_length=s)
    raise ValueError("modelo desconocido: " + fam)


NOMBRES = {"theta": "Theta", "croston": "Croston", "ingenuo": "Ingenuo", "ingenuo_estacional": "Ingenuo estacional", "tbats": "TBATS"}


def nombre(mod, fam):
    try:
        if fam == "arima":
            from statsforecast.arima import arima_string

            return " ".join(arima_string(mod.model_).split())
        if fam == "ets":
            return str(mod.model_["method"]).strip()
    except Exception:
        pass
    return NOMBRES.get(fam, type(mod).__name__)


def main():
    d = json.load(sys.stdin)
    y_all = np.asarray(d["y"], dtype=float)
    m = float(d["m"])
    nivel = int(round(d["nivel"]))
    lam = d.get("lambda")
    out = []
    for t in d["trabajos"]:
        try:
            fam = t["familia"]
            usa_lam = fam in ("arima", "ets", "theta", "ingenuo", "ingenuo_estacional")
            la = lam if usa_lam else None
            y = boxcox(y_all[: t["n"]], la)
            mod = modelo(fam, t.get("extra", {}), m)
            mod = mod.fit(y=y)
            fc = mod.predict(h=t["h"], level=[nivel]) if fam != "croston" else mod.predict(h=t["h"])
            media = inv_boxcox(fc["mean"], la)
            inf = inv_boxcox(fc.get(f"lo-{nivel}", [float("nan")] * t["h"]), la)
            sup = inv_boxcox(fc.get(f"hi-{nivel}", [float("nan")] * t["h"]), la)
            try:
                ajust = inv_boxcox(mod.predict_in_sample()["fitted"], la)
            except Exception:
                ajust = [float("nan")] * t["n"]
            r = {"modelo": nombre(mod, fam), "media": lista(media), "inferior": lista(inf), "superior": lista(sup), "ajustados": lista(ajust), "coef": {}}
            mm = getattr(mod, "model_", None)
            if isinstance(mm, dict):
                for k in ("aic", "aicc", "bic", "sigma2"):
                    v = mm.get(k)
                    if isinstance(v, (int, float)) and math.isfinite(v):
                        r[k] = float(v)
                coef = mm.get("coef")
                if isinstance(coef, dict):
                    r["coef"] = {k: float(v) for k, v in coef.items() if isinstance(v, (int, float))}
                par = mm.get("par")
                if isinstance(par, dict):
                    r["coef"].update({k: float(v) for k, v in par.items() if isinstance(v, (int, float)) and math.isfinite(v)})
            out.append(r)
        except Exception as e:  # cada trabajo falla por separado
            out.append({"error": str(e)})
    json.dump(out, sys.stdout)


if __name__ == "__main__":
    main()
