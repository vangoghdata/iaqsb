---
name: crear-curso-nativo
description: Crea o modifica un curso de Nativo (.ncourse) a partir de diapositivas, libros, apuntes o un programa de ejemplo — lecciones, calculadoras en Python o R y modelos de optimización en el lenguaje algebraico — y lo valida, prueba y empaqueta para compartirlo. Úsala cuando alguien pida construir un tema o un curso para Nativo (antes IAQSB).
---

# Crear un curso de Nativo

Nativo es un programa para aprender y aplicar la ingeniería industrial. Un **curso** (`.ncourse`) es un paquete de temas que se instala en Nativo; cada tema aparece como un módulo con su pantalla de datos, su informe, su paso a paso y su guía. El formato completo está en `docs/CURSOS.md` del repositorio de Nativo (léelo antes de empezar) y el curso de muestra en `cursos/ejemplo-nativo`.

## Antes de escribir

1. Pregunta, si no lo dijeron: el tema o los temas, el nivel (pregrado, posgrado, empresa), el idioma y los materiales de referencia (diapositivas, libros en PDF, Excel o un programa viejo cuyo cálculo hay que reproducir).
2. Lee los materiales y extrae: la definición, el procedimiento paso a paso, las fórmulas, los datos de los ejemplos resueltos y sus resultados. Si hay un Excel o un programa de ejemplo, el cálculo del tema debe dar **exactamente** sus números.
3. Decide la clase de cada tema: `leccion` (solo teoría), `calculadora` (Python o R) o `modelo` (optimización en el lenguaje algebraico, resuelta con HiGHS).

## El ciclo

```text
iaqsb curso nuevo <carpeta> --id <id> --titulo "<título>"     plantilla con un tema de lección y uno de calculadora
(escribe curso.json, los temas, las lecciones, la pantalla, el cálculo y los ejemplos)
iaqsb curso validar <carpeta>                                  corrige todos los errores
iaqsb --confiar curso probar <carpeta>                         resuelve los ejemplos: compara con los resultados de los materiales
iaqsb curso empaquetar <carpeta>                               crea <id>.ncourse
iaqsb curso instalar <id>.ncourse                              para verlo en la app
```

Con el servidor MCP de Nativo, la herramienta `courses` hace lo mismo (action = new, validate, test, pack, install).

## Reglas

- **Exactitud:** cada ejemplo debe reproducir el resultado de su fuente (libro, diapositiva, Excel). Si no hay resultado publicado, compáralo con otra implementación independiente (otra fórmula, R, Python, el módulo equivalente de Nativo).
- **Fuentes:** cada fórmula, procedimiento o dato tomado de un material va en `fuentes.json` con su referencia en APA 7 (`id`, `cita`, `apa`). Saca los datos bibliográficos de la página legal del libro. **Nunca inventes una referencia**: si no puedes confirmarla, pon `"verificada": false`.
- **Derechos de autor:** no copies páginas ni párrafos de los libros. Explica con tus palabras, cita la fuente y usa citas textuales cortas. Los datos de un ejercicio sí se pueden usar, con su cita.
- **Pantalla:** usa los mismos nombres de campos en `pantalla.json`, en los ejemplos y en el código. Cada campo con una etiqueta clara y, si hace falta, una ayuda (`help`) con unidades.
- **Informe:** un titular con el resultado principal en una frase; métricas con unidades; tablas y gráficas que se entiendan solas; un paso a paso con las fórmulas aplicadas a los datos; una interpretación de qué decidir con el resultado.
- **Lecciones:** qué es, para qué sirve en la vida real, el procedimiento, las fórmulas (en Unicode: x̄, σ², √, Σ, ≤), un ejemplo, errores comunes y cómo se hace hoy en la industria. Tono profesional y educativo, frases cortas, sin emojis.
- **Python en la app** corre con Pyodide (numpy, pandas, scipy, statsmodels, scikit-learn, networkx, sympy, matplotlib); no uses paquetes que no estén ahí ni acceso a red o a archivos fuera de la carpeta del tema.
- Termina solo cuando `validar` no da errores y `probar` da bien en todos los ejemplos. Di qué quedó por verificar.
