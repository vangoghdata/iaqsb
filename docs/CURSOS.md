# Cursos de Nativo (`.ncourse`)

Un **curso** es un paquete de temas que cualquiera puede crear, compartir e instalar: un profesor arma el curso de su materia, un estudiante hace el tema que le faltaba, una IA lo construye a partir de diapositivas o libros. Al instalarlo, cada tema aparece en la barra lateral de Nativo como un módulo más, con su pantalla de datos, su informe, su paso a paso y su guía.

Hay tres clases de temas:

| Tipo | Qué tiene | Ejemplo |
| --- | --- | --- |
| `leccion` | Lecciones en Markdown y una guía (qué es, historia, para qué sirve…) | Qué es un ERP y cómo se usa |
| `calculadora` | Una pantalla de datos y un cálculo en **Python** o en **R** | Punto de equilibrio, promedios móviles |
| `modelo` | Una pantalla de datos y un modelo de optimización en el **lenguaje algebraico** de Nativo, resuelto con HiGHS | Mezcla de productos |

El curso de muestra está en [`cursos/ejemplo-nativo`](../cursos/ejemplo-nativo): sirve de plantilla.

## Estructura

```text
mi-curso/
  curso.json                   el curso
  fuentes.json                 referencias propias (opcional)
  temas/
    <tema>/
      tema.json                el tema
      guia.json                guía con el formato de Guíame (opcional)
      leccion.md               lecciones en Markdown (opcional)
      pantalla.json            la pantalla de datos (calculadora y modelo)
      calcular.py | calcular.R | modelo.iaqsb
      ejemplos/*.json          datos de ejemplo
```

El archivo `.ncourse` es esa carpeta comprimida en zip. Se aceptan archivos `json`, `md`, `txt`, `py`, `R`, `iaqsb`, `csv`, `tsv`, `xlsx`, imágenes y `pdf`, hasta 60 MB.

### `curso.json`

```json
{
  "formato": 1,
  "id": "produccion-basica",
  "titulo": "Producción básica",
  "version": "1.0.0",
  "descripcion": "Qué enseña el curso y para quién es.",
  "autores": [{ "nombre": "Ana Pérez", "institucion": "Universidad Nacional de Colombia" }],
  "licencia": "CC BY 4.0",
  "idioma": "es",
  "area": "Producción y logística",
  "temas": ["que-es-un-erp", "punto-de-equilibrio"]
}
```

`id` y los ids de los temas: de 2 a 48 letras minúsculas, números o guiones. El orden de `temas` es el orden en la barra lateral. Al instalar una versión nueva de un curso con el mismo `id`, reemplaza a la anterior.

### `temas/<tema>/tema.json`

```json
{
  "id": "punto-de-equilibrio",
  "titulo": "Punto de equilibrio",
  "resumen": "Una o dos frases: qué decide y para quién.",
  "tipo": "calculadora",
  "lecciones": [{ "titulo": "El punto de equilibrio", "archivo": "leccion.md" }],
  "calculo": { "lenguaje": "python", "archivo": "calcular.py", "pantalla": "pantalla.json" },
  "ejemplos": [{ "nombre": "Taller de muebles", "archivo": "ejemplos/muebles.json" }]
}
```

`calculo.lenguaje` es `python`, `r` o `algebra`. Un tema `leccion` no lleva `calculo`.

### `pantalla.json`: la pantalla de datos

Es la misma especificación que usan los módulos de Nativo: uno o varios tipos de problema, cada uno con bloques de campos.

```json
{
  "problem_types": [{
    "id": "equilibrio", "label": "Punto de equilibrio", "description": "…", "dims": [],
    "layout": [
      { "type": "params", "title": "Datos", "fields": [
        { "key": "precio", "label": "Precio de venta", "kind": { "type": "number" }, "default": 10, "help": "Por unidad" }
      ]},
      { "type": "records", "title": "Productos", "path": "productos", "add_label": "Añadir producto", "columns": [
        { "key": "nombre", "label": "Producto", "kind": { "type": "text" } },
        { "key": "utilidad", "label": "Utilidad", "kind": { "type": "number" } }
      ]}
    ]
  }],
  "options": []
}
```

- `params`: campos sueltos; quedan en el modelo con su `key` (`modelo["precio"]`).
- `records`: una tabla; queda como lista de filas en `path` (`modelo["productos"]`).
- Tipos de campo (`kind.type`): `number`, `integer`, `text`, `bool`, `choice` (con `options: [["valor", "Etiqueta"], …]`), `list`.
- `show_if: [["campo", ["valor1", "valor2"]]]` muestra un campo solo con esos valores de otro.

Los ejemplos (`ejemplos/*.json`) son modelos completos: `{"name": "…", "problem_type": "equilibrio", "precio": 180000, …}`.

### El cálculo en Python

```python
from nativo import modelo, Informe

p = float(modelo["precio"])
inf = Informe(f"Resultado principal en una frase")
inf.metricas([("Punto de equilibrio", 171.4, "mesas"), ("Margen", 70000)])
inf.tabla(["Unidades", "Utilidad"], [[0, -12000000], [171, 0]], titulo="Tabla", tonos=["bad", "good"])
inf.grafica({"Ingresos": [(0, 0), (300, 54000000)]}, titulo="Ingresos", x="Unidades", y="Pesos")
inf.texto("Interpretación…", tono="good")
inf.paso("Fórmula", "Q* = CF / (p − cv) = …")
inf.datos(punto_equilibrio=171.4)
inf.publicar()
```

`nativo.py` lo pone Nativo: no va en el curso. En la app de escritorio el código corre **aislado** con Pyodide (Python en WebAssembly, con numpy, pandas, scipy, statsmodels, scikit-learn, networkx, sympy y matplotlib): no puede leer ni escribir los archivos del equipo. En la línea de comandos y en el servidor MCP corre con el Python del equipo solo con `iaqsb --confiar` (o `IAQSB_CONFIAR_CODIGO=1`).

### El cálculo en R

```r
inf <- informe("Resultado principal")
inf <- metricas(inf, list(list("Pronóstico", 151)))
inf <- tabla(inf, c("Periodo", "Valor"), list(list("Ene", 120), list("Feb", 132)), titulo = "Datos")
inf <- grafica(inf, list("Real" = c(120, 132, 125)), titulo = "Serie", x = "Periodo", y = "Valor")
inf <- paso(inf, "Fórmula", "F(t+1) = …")
publicar(inf)
```

Los datos están en `modelo` (listas de R). No hace falta ningún paquete; si el código usa otros (`library(forecast)`), en la app se instalan solos con webR.

### El modelo algebraico

`modelo.iaqsb` es un modelo del [lenguaje algebraico](LENGUAJE.md) con huecos `{{…}}` que se llenan con los datos de la pantalla:

- `{{campo}}`: un dato suelto (`{{horas_maquina}}`).
- `{{tabla.columna}}`: la columna de una tabla, separada por comas (`set P = { {{productos.nombre}} };` y `param u{P} = [ {{productos.utilidad}} ];`).

Se resuelve con HiGHS y el informe trae las variables por índices, las restricciones, los precios sombra y el modelo ya lleno.

### `guia.json` y `fuentes.json`

La guía tiene el formato de las guías de los módulos ([`crates/iaqsb-core/guias/ESQUEMA.md`](../crates/iaqsb-core/guias/ESQUEMA.md)); todos los campos son opcionales. Las referencias son ids de la bibliografía de Nativo ([FUENTES.md](FUENTES.md)) o de `fuentes.json` del curso:

```json
[{ "id": "chopra2019", "cita": "Chopra y Meindl, 2019", "apa": "Chopra, S., y Meindl, P. (2019). *Supply chain management* (7.ª ed.). Pearson.", "verificada": true }]
```

Nunca inventes referencias: si no puedes confirmar una, ponle `"verificada": false`. No copies páginas de libros: explica con tus palabras, cita la fuente y usa citas textuales cortas.

## Comandos

```text
iaqsb curso nuevo mi-curso --id produccion-basica --titulo "Producción básica"
iaqsb curso validar mi-curso
iaqsb --confiar curso probar mi-curso      resuelve los ejemplos de cada tema
iaqsb curso empaquetar mi-curso            crea produccion-basica.ncourse
iaqsb curso instalar produccion-basica.ncourse
iaqsb curso lista
iaqsb curso quitar produccion-basica
```

En la app: **Instalar un curso…** en la barra lateral, arrastrar el `.ncourse` a la ventana o abrirlo con doble clic. Los cursos instalados son los mismos para la app, la CLI y el servidor MCP (en la carpeta de datos de Nativo).

## Crear un curso con IA

- **En la app (asistente de creación):** **Crear un curso…** en la barra lateral. Creas un borrador (vacío, con temas de ejemplo, copia de un curso instalado o abriendo un `.ncourse` que te compartieron) y Nativo lo arma contigo: le adjuntas en el chat tus diapositivas (`.pptx`), libros o apuntes (PDF, `.docx`), programas (`.py`, `.R`) o datos (`.xlsx`), le dices qué tema quieres enseñar y qué puede usar (lecciones, Python, R, modelos de optimización), y él escribe los archivos del curso, los valida, resuelve los ejemplos y te abre la vista previa. Puedes editar cualquier archivo a mano en el mismo editor, agregar temas con plantillas que ya funcionan, probar los ejemplos, ver el curso como lo verán los estudiantes (**Vista previa**) y **Exportar .ncourse** para compartirlo. Necesita un modelo de IA conectado en Ajustes (Claude, ChatGPT, Gemini, Qwen, uno local con Ollama…); sin modelo, el editor, las plantillas, la validación, las pruebas y la exportación funcionan igual. Los borradores quedan en la carpeta de datos de Nativo (`borradores/`), al lado de los cursos instalados.
- **Con Claude Code, Codex u otro agente:** la skill [`skills/crear-curso-nativo`](../skills/crear-curso-nativo/SKILL.md) le explica el formato y el ciclo (crear, escribir, validar, probar, empaquetar). Dale tus diapositivas, libros o un programa de ejemplo y dile qué tema construir.
- **Con el servidor MCP:** la herramienta `courses` crea, valida, prueba, empaqueta e instala cursos.
