# Guías de los módulos («Guíame»)

Cada módulo de IAQSB tiene una guía en `crates/iaqsb-core/guias/<id>.json` (por ejemplo `mrp.json`). La app la muestra con el botón **Guíame**, y la CLI y el servidor MCP la entregan con la operación `guide`. Está pensada para que un estudiante pueda aprender el tema solo con IAQSB, y para que un profesor la use en clase en lugar de los programas viejos (WinQSB, Jantero).

Tono: profesional, claro, empresarial y educativo, como un buen profesor universitario que además conoce la industria. En español de Colombia neutro, tuteando. Sin emojis ni chistes. Frases cortas, de una idea cada una.

## Formato

```json
{
  "modulo": "mrp",
  "titulo": "Planeación de requerimientos de materiales (MRP)",
  "resumen": "Una o dos frases: qué decide y para quién.",
  "que_es": ["Párrafo 1: la definición, sin jerga.", "Párrafo 2: la intuición y los conceptos clave."],
  "analogia": "Una comparación con algo cotidiano que ayude a entender la idea.",
  "para_que_sirve": [
    { "sector": "Manufactura de muebles", "ejemplo": "Qué decisión real resuelve y qué gana la empresa." }
  ],
  "historia": [
    { "anio": "1964", "hecho": "Qué pasó y por qué importa.", "refs": ["orlicky1975"] }
  ],
  "necesitas": [
    { "dato": "Programa maestro de producción", "para_que": "Para qué lo usa el método y de dónde sale en una empresa." }
  ],
  "supuestos": ["Supuesto o límite del modelo."],
  "pasos": [
    { "titulo": "Elige el tipo de problema", "texto": "Qué hacer en la pantalla de IAQSB, con los nombres reales de los campos." }
  ],
  "como_resuelve": ["Qué método o algoritmo usa IAQSB y con qué motor (HiGHS, R, etc.), en lenguaje claro, y cómo leer el paso a paso."],
  "interpretar": ["Cómo leer los resultados del informe y qué decisión tomar con ellos."],
  "industria": [
    { "nombre": "SAP S/4HANA (MRP Live)", "tipo": "software", "uso": "Para qué lo usan las empresas hoy." },
    { "nombre": "Google OR-Tools", "tipo": "libreria", "uso": "Qué resuelve y en qué lenguaje se usa." }
  ],
  "alternativas": ["Otros métodos o enfoques para el mismo problema y cuándo convienen (por ejemplo DDMRP frente a MRP)."],
  "errores_comunes": ["Error típico de estudiantes o de empresas, y cómo evitarlo."],
  "profesor": {
    "objetivos": ["Qué debe saber hacer el estudiante al terminar."],
    "actividades": ["Actividad de clase o taller con IAQSB (con el ejemplo incluido o datos propios)."],
    "preguntas": ["Pregunta de evaluación o de discusión."]
  },
  "glosario": [{ "termino": "Explosión de materiales", "definicion": "Definición en una frase." }],
  "referencias": ["orlicky1975", "hillier2015en"]
}
```

## Reglas

- Las referencias (`historia[].refs` y `referencias`) son **identificadores** de `crates/iaqsb-core/fuentes/bibliografia.json`. Usa primero los que ya existen.
- **Nunca inventes una referencia.** Si hace falta una nueva, agrégala en `crates/iaqsb-core/guias/_nuevas_<grupo>.json` (una lista con el mismo formato de las entradas de `bibliografia.json`). Compruébala en internet (DOI, editorial, catálogo de biblioteca) y ponle `"estado": "verificada"` solo si la confirmaste; si no, `"estado": "por_verificar"` y en `"pendiente"` lo que falta confirmar.
- Los años y los hechos de la historia deben poder respaldarse con la referencia que citan. Si no tienes fuente para un hecho, quítalo.
- `pasos` debe coincidir con la pantalla real del módulo: usa los nombres de los tipos de problema, los campos y las opciones que da `iaqsb --json describir <id>` (o `iaqsb --json llamar describe_module` con `{"module": "<id>"}`), y los ejemplos incluidos (`iaqsb ejemplos <id>`). Termina con «Resuelve» y cómo ver el paso a paso.
- `industria`: software y librerías que de verdad se usan hoy (2026). Describe para qué sirven sin inventar versiones, precios ni cifras de mercado. `tipo` es `software`, `libreria`, `servicio` o `metodologia`.
- Largo orientativo: 1200 a 2000 palabras por guía. Mejor claro y correcto que largo.
