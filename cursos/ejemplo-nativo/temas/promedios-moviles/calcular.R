# Promedio móvil de n periodos, con sus errores. `modelo` trae los datos de la pantalla.
n <- as.integer(if (is.null(modelo$n)) 3 else modelo$n)
y <- vapply(modelo$datos, function(d) as.numeric(d$valor), numeric(1))
etq <- vapply(modelo$datos, function(d) as.character(d$periodo), character(1))
k <- length(y)

if (n < 1 || n >= k) {
  inf <- informe(sprintf("n debe estar entre 1 y %d (hay %d datos)", k - 1, k), estado = "infeasible")
  inf <- texto(inf, "Escribe más datos o un n más pequeño.")
} else {
  f <- rep(NA_real_, k + 1)
  for (t in (n + 1):(k + 1)) f[t] <- mean(y[(t - n):(t - 1)])
  e <- y - f[1:k]
  ok <- !is.na(e)
  mad <- mean(abs(e[ok]))
  mse <- mean(e[ok]^2)
  mape <- mean(abs(e[ok] / y[ok])) * 100
  sig <- f[k + 1]

  inf <- informe(sprintf("Pronóstico del próximo periodo: %.2f (promedio de los últimos %d)", sig, n))
  inf <- metricas(inf, list(list("Pronóstico siguiente", round(sig, 4)), list("MAD", round(mad, 4)), list("MSE", round(mse, 4)), list("MAPE", round(mape, 2), "%")))
  filas <- lapply(1:k, function(t) list(etq[t], y[t], if (is.na(f[t])) NULL else round(f[t], 4), if (is.na(e[t])) NULL else round(e[t], 4)))
  filas[[k + 1]] <- list("Siguiente", NULL, round(sig, 4), NULL)
  inf <- tabla(inf, c("Periodo", "Real", "Pronóstico", "Error"), filas, titulo = "Pronósticos y errores")
  inf <- grafica(inf, list("Real" = lapply(1:k, function(t) c(t - 1, y[t])), "Pronóstico" = lapply((n + 1):(k + 1), function(t) c(t - 1, f[t]))), titulo = "Serie y promedio móvil", x = "Periodo", y = "Valor")
  inf <- paso(inf, "Promedio móvil", sprintf("F(t+1) = promedio de los últimos %d datos. El siguiente: (%s) / %d = %.4f", n, paste(y[(k - n + 1):k], collapse = " + "), n, sig))
  inf <- paso(inf, "Errores", sprintf("MAD = Σ|e| / %d = %.4f; MSE = Σe² / %d = %.4f; MAPE = %.2f %%", sum(ok), mad, sum(ok), mse, mape))
  inf <- datos(inf, pronostico = sig, mad = mad, mse = mse, mape = mape)
}
publicar(inf)
