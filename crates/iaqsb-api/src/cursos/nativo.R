# Ayuda para los temas de cursos de Nativo (.ncourse) que calculan en R. No necesita paquetes.
#
#   inf <- informe("Punto de equilibrio: 1250 unidades")
#   inf <- metricas(inf, list(list("Punto de equilibrio", 1250, "unidades")))
#   inf <- tabla(inf, c("Unidades", "Ingreso"), list(list(0, 0), list(1250, 12500)), titulo = "Tabla")
#   inf <- grafica(inf, list("Ingreso" = list(c(0, 0), c(2500, 25000))), titulo = "Equilibrio", x = "Unidades", y = "Pesos")
#   inf <- paso(inf, "Fórmula", "Q* = CF / (p - cv)")
#   publicar(inf)
#
# Los datos de la pantalla están en `modelo` y las opciones en `opciones` (listas de R).
source("entrada.R", encoding = "UTF-8")
modelo <- if (is.null(.entrada$model)) list() else .entrada$model
opciones <- if (is.null(.entrada$options)) list() else .entrada$options
idioma <- if (is.null(.entrada$lang)) "es" else .entrada$lang

.num <- function(v) {
  if (is.null(v) || length(v) == 0) return(NULL)
  if (is.numeric(v) && !is.finite(v)) return(NULL)
  if (is.numeric(v) || is.logical(v) || is.character(v)) return(v)
  as.character(v)
}

informe <- function(titular, estado = "solved") {
  list(titular = as.character(titular), estado = estado, secciones = list(), pasos = list(), datos = list())
}

.poner <- function(inf, seccion, bloque) {
  if (is.null(inf$secciones[[seccion]])) {
    titulo <- if (seccion == "solucion") "Solución" else seccion
    inf$secciones[[seccion]] <- list(id = seccion, title = titulo, blocks = list())
  }
  inf$secciones[[seccion]]$blocks[[length(inf$secciones[[seccion]]$blocks) + 1]] <- bloque
  inf
}

metricas <- function(inf, items, titulo = NULL, seccion = "solucion") {
  lista <- lapply(seq_along(items), function(k) {
    it <- items[[k]]
    m <- list(label = as.character(it[[1]]), value = .num(it[[2]]))
    if (length(it) > 2) m$unit <- as.character(it[[3]])
    if (k == 1) m$emphasis <- TRUE
    m
  })
  b <- list(type = "metrics", items = lista)
  if (!is.null(titulo)) b$title <- titulo
  .poner(inf, seccion, b)
}

tabla <- function(inf, columnas, filas, titulo = NULL, nota = NULL, seccion = "solucion") {
  cols <- lapply(columnas, function(c) list(label = as.character(c)))
  rows <- lapply(filas, function(f) list(cells = lapply(f, function(x) if (is.null(x)) NULL else .num(x))))
  for (k in seq_along(cols)) {
    if (any(vapply(filas, function(f) length(f) >= k && is.numeric(f[[k]]), logical(1)))) cols[[k]]$align <- "right"
  }
  b <- list(type = "table", columns = cols, rows = rows)
  if (!is.null(titulo)) b$title <- titulo
  if (!is.null(nota)) b$note <- nota
  .poner(inf, seccion, b)
}

texto <- function(inf, cuerpo, titulo = NULL, seccion = "solucion") {
  b <- list(type = "text", body = as.character(cuerpo))
  if (!is.null(titulo)) b$title <- titulo
  .poner(inf, seccion, b)
}

grafica <- function(inf, series, titulo = NULL, x = "", y = "", tipo = "line", seccion = "solucion") {
  lista <- lapply(names(series), function(n) {
    pts <- series[[n]]
    if (is.numeric(pts)) pts <- lapply(seq_along(pts), function(i) c(i - 1, pts[[i]]))
    list(name = n, kind = tipo, points = lapply(pts, function(p) list(.num(p[[1]]), .num(p[[2]]))))
  })
  b <- list(type = "chart", x_label = x, y_label = y, series = lista)
  if (!is.null(titulo)) b$title <- titulo
  .poner(inf, seccion, b)
}

paso <- function(inf, titulo, descripcion = "") {
  inf$pasos[[length(inf$pasos) + 1]] <- list(title = as.character(titulo), description = as.character(descripcion), blocks = list())
  inf
}

datos <- function(inf, ...) {
  inf$datos <- c(inf$datos, list(...))
  inf
}

# JSON sin paquetes
.json_texto <- function(s) {
  s <- enc2utf8(as.character(s))
  s <- gsub("\\", "\\\\", s, fixed = TRUE)
  s <- gsub("\"", "\\\"", s, fixed = TRUE)
  s <- gsub("\n", "\\n", s, fixed = TRUE)
  s <- gsub("\r", "", s, fixed = TRUE)
  s <- gsub("\t", "\\t", s, fixed = TRUE)
  paste0("\"", s, "\"")
}

.json <- function(x) {
  if (is.null(x)) return("null")
  if (is.list(x)) {
    n <- names(x)
    if (!is.null(n) && length(x) == 0) return("{}")
    if (!is.null(n) && all(nzchar(n))) {
      partes <- vapply(seq_along(x), function(i) paste0(.json_texto(n[[i]]), ":", .json(x[[i]])), character(1))
      return(paste0("{", paste0(partes, collapse = ","), "}"))
    }
    if (length(x) == 0) return("[]")
    return(paste0("[", paste0(vapply(x, .json, character(1)), collapse = ","), "]"))
  }
  if (length(x) == 0) return("[]")
  if (length(x) > 1) return(.json(as.list(x)))
  if (is.na(x)) return("null")
  if (is.logical(x)) return(if (x) "true" else "false")
  if (is.numeric(x)) return(if (is.finite(x)) format(x, digits = 15, scientific = FALSE, trim = TRUE) else "null")
  .json_texto(x)
}

publicar <- function(inf) {
  secciones <- unname(inf$secciones)
  if (length(inf$pasos)) secciones[[length(secciones) + 1]] <- list(id = "pasos", title = "Paso a paso", blocks = list(list(type = "steps", steps = inf$pasos)))
  r <- list(module = "", problem = if (is.null(modelo$name)) "" else as.character(modelo$name), status = inf$estado, headline = inf$titular, sections = secciones, data = if (length(inf$datos)) inf$datos else structure(list(), names = character(0)))
  cat("@@NATIVO_INFORME@@", .json(r), "\n", sep = "")
}
