# Lenguaje de modelado algebraico de IAQSB

Referencia del lenguaje con el que IAQSB escribe modelos de programación lineal, entera y cuadrática con conjuntos e índices, al estilo de AMPL y LINGO. Las palabras clave se aceptan en español, inglés y portugués, y se pueden mezclar en un mismo modelo.

Todo lo que se describe aquí sale del código (`crates/iaqsb-lp/src/algebra/`) y cada modelo completo de este documento (los bloques marcados como `iaqsb`) se comprobó con la línea de comandos: se revisa y se resuelve tal como está escrito.

## Contenido

1. [Introducción](#1-introducción)
2. [Modo rápido](#2-modo-rápido)
3. [Estructura de un modelo](#3-estructura-de-un-modelo)
4. [Conjuntos](#4-conjuntos)
5. [Parámetros y datos](#5-parámetros-y-datos)
6. [Variables: cotas y tipos](#6-variables-cotas-y-tipos)
7. [Objetivos](#7-objetivos)
8. [Restricciones](#8-restricciones)
9. [Sumatorias y condiciones](#9-sumatorias-y-condiciones)
10. [Restricciones generales](#10-restricciones-generales)
11. [Funciones de datos](#11-funciones-de-datos)
12. [Palabras clave en español, inglés y portugués](#12-palabras-clave-en-español-inglés-y-portugués)
13. [Comentarios, números y textos](#13-comentarios-números-y-textos)
14. [Errores frecuentes](#14-errores-frecuentes)
15. [Resultados: tablas por índices](#15-resultados-tablas-por-índices)
16. [Uso desde la línea de comandos](#16-uso-desde-la-línea-de-comandos)
17. [Uso desde MCP y la API JSON](#17-uso-desde-mcp-y-la-api-json)
18. [Conversión desde la hoja y desde WinQSB](#18-conversión-desde-la-hoja-y-desde-winqsb)

## 1. Introducción

### Para qué sirve

El módulo de programación lineal y entera (LP-ILP) tiene dos formas de escribir un modelo:

- **La hoja**, igual a la de WinQSB: una columna por variable y una fila por restricción. Es la forma natural para los ejercicios de clase con pocas variables, y es la que habilita el modo práctica, el paso a paso con las tablas del símplex y el panel «¿Qué pasa si…?».
- **El modelo algebraico** (este lenguaje): se declaran conjuntos, parámetros y variables indexadas, y las restricciones se escriben una sola vez para todos los índices. Cien plantas y mil clientes ocupan las mismas líneas que tres plantas y cinco clientes.

Un transporte de 3 plantas y 5 ciudades se escribe así (es el primer ejemplo del editor, en *Ejemplos*):

```iaqsb
# Transporte con índices: 3 plantas abastecen a 5 ciudades al menor costo.
conjunto Plantas  = {Cali, Medellin, Bogota};
conjunto Ciudades = {Pasto, Pereira, Tunja, Neiva, Ibague};

parametro oferta{Plantas}   = [300, 500, 400];
parametro demanda{Ciudades} = [180, 220, 160, 240, 200];
parametro costo{Plantas, Ciudades} = [
  [4, 6, 9, 5, 7],
  [7, 3, 6, 6, 4],
  [9, 5, 2, 4, 5]
];

variable x{Plantas, Ciudades} >= 0;

minimizar costo_total: suma{i en Plantas, j en Ciudades} costo[i,j] * x[i,j];

sujeto a
  capacidad{i en Plantas}: suma{j en Ciudades} x[i,j] <= oferta[i];
  atender{j en Ciudades}:  suma{i en Plantas} x[i,j] >= demanda[j];
```

El óptimo es 3460 y la solución se muestra como tabla de doble entrada plantas × ciudades (sección 15).

### Cuándo usar la hoja y cuándo el lenguaje

| Situación | Conviene |
| --- | --- |
| Ejercicio de clase con 2 a 10 variables; se quiere ver el símplex, practicarlo o dibujar el método gráfico | Hoja |
| Abrir o entregar un archivo `.LPP` de WinQSB | Hoja (el lenguaje también exporta a `.LPP`, ver sección 18) |
| Modelos con familias de variables (`x[i,j]`, `inv[p,t]`), datos en tablas, muchos periodos o muchos nodos | Lenguaje |
| Costos fijos, valores absolutos, máximos, funciones por tramos, restricciones lógicas o disyuntivas | Lenguaje (las reformula solo, sección 10) |
| Modelos que se versionan en git, se revisan en clase o se resuelven por lotes | Lenguaje (son archivos de texto `.iaqsb`) |
| Objetivo cuadrático convexo con variables continuas | Lenguaje; para casos no convexos o enteros, el módulo QP-IQP |

En la aplicación, el selector **Hoja | Modelo algebraico** del módulo LP-ILP cambia entre las dos formas. El botón **Pasar a la hoja** expande el modelo (una columna por cada `x[i,j]`) y **Desde la hoja** escribe la hoja actual en el lenguaje. La práctica y «¿Qué pasa si…?» se usan desde la hoja; en el modo algebraico no aparecen.

El editor revisa el texto mientras se escribe (señala el error con su línea y columna), resuelve con <kbd>Ctrl</kbd>+<kbd>Intro</kbd>, comenta o descomenta líneas con <kbd>Ctrl</kbd>+<kbd>/</kbd> y trae una **Ayuda del lenguaje** y un menú de **Ejemplos** (transporte, producción por periodos, localización con binarias, secuenciación con disyunciones, descuentos por cantidad y modo rápido). Si el modelo resuelto tiene exactamente dos variables continuas y objetivo lineal, se dibuja además el método gráfico.

## 2. Modo rápido

Si el texto no tiene ninguna declaración `variable` (o `var`), cada nombre suelto que aparece en el objetivo o en las restricciones se toma como una variable continua mayor o igual que cero. Sirve para los ejercicios de dos o tres variables:

```iaqsb
# Modo rápido: x, y se toman como variables continuas >= 0.
max 3x + 5y;
s.t.
planta1: x <= 4;
planta2: 2y <= 12;
planta3: 3x + 2y <= 18;
```

El informe lo advierte con la nota «Sin declaraciones var: x, y se tomaron como variables continuas ≥ 0». Se admite la notación de los libros: nombre del objetivo con `=`, `sujeto a:` con dos puntos, restricciones sin nombre y multiplicación implícita (`3x1`, `2x2`):

```iaqsb
Maximizar Z = 3x1 + 5x2;
sujeto a:
  x1 <= 4;
  2x2 <= 12;
  3x1 + 2x2 <= 18;
```

Reglas del modo rápido:

- En cuanto hay **una** declaración `variable`, el modo rápido se apaga y todo nombre sin declarar es un error (««w» no está definido»). Así se detectan las erratas en los modelos grandes.
- Las variables automáticas no llevan índices: `x[1]` exige declarar `x`.
- Son continuas y mayores o iguales que cero. Para cotas, enteras, binarias o libres hay que declararlas (sección 6).

## 3. Estructura de un modelo

Un modelo es una lista de instrucciones que terminan en `;`. Hay seis clases de instrucciones:

| Instrucción | Forma | Sección |
| --- | --- | --- |
| Conjunto | `conjunto I = {A, B, C};` | 4 |
| Parámetro | `parametro c{I} = [5, 4, 3];` | 5 |
| Variable | `variable x{I} >= 0;` | 6 |
| Objetivo | `maximizar z: suma{i en I} c[i] * x[i];` | 7 |
| Restricción | `cap: suma{i en I} x[i] <= 10;` | 8 |
| Opción | `opcion bigM = 10000;` | 10 |

Los nombres se definen antes de usarse: un conjunto antes de los parámetros y variables que lo usan, un parámetro antes de la restricción que lo lee. `sujeto a` (o `subject to`, `s.t.`, `s.a.`) es opcional: puede ir una vez antes de todas las restricciones, delante de cada una o no aparecer.

```iaqsb
# Mezcla de productos: tres productos comparten 100 horas de máquina.
conjunto I = {A, B, C};
parametro ganancia{I} = [5, 4, 3];
parametro horas{I}    = [2, 3, 1];
parametro tope{I}     = [30, 20, 40];

variable x{I} >= 0;

maximizar z: suma{i en I} ganancia[i] * x[i];

sujeto a
  maquina: suma{i en I} horas[i] * x[i] <= 100;
  mercado{i en I}: x[i] <= tope[i];
```

El mismo modelo en inglés y en portugués:

```iaqsb
set Products = {Door, Window};
param profit{Products} = [3, 5];
var x{Products} >= 0;
maximize z: sum{p in Products} profit[p] * x[p];
subject to
  plant1: x[Door] <= 4;
  plant2: 2 * x[Window] <= 12;
  plant3: 3 * x[Door] + 2 * x[Window] <= 18;
```

```iaqsb
conjunto Produtos = {P1, P2};
parametro lucro{Produtos} = [3, 5];
variavel x{Produtos} >= 0;
maximizar z: soma{p em Produtos} lucro[p] * x[p];
sujeito a
  r1: x[P1] <= 4;
  r2: 2 * x[P2] <= 12;
  r3: 3 * x[P1] + 2 * x[P2] <= 18;
```

Sobre las mayúsculas y las tildes: en las **palabras clave** no importan (`Sujeto A`, `parámetro`, `SUMA` valen). En los **nombres** sí: `x` y `X` son variables distintas, y el elemento `Bogotá` no es `Bogota`.

## 4. Conjuntos

Un conjunto es una lista de elementos sin repetir, en un orden fijo. Ese orden es el que siguen las listas de datos (sección 5) y las filas y columnas de los resultados.

| Forma | Ejemplo | Resultado |
| --- | --- | --- |
| Lista de nombres | `conjunto Nodos = {A, B, C, D};` | A, B, C, D |
| Rango de enteros | `conjunto Meses = 1..12;` | 1, 2, …, 12 |
| Rango entre llaves | `conjunto Semanas = {1..4};` | 1, 2, 3, 4 |
| Textos con espacios o signos | `conjunto Ciudades = {"San José", Lima};` | San José, Lima |
| Pares (o tuplas) | `conjunto Arcos = {(A,B), (A,C), (B,D)};` | conjunto de pares |
| Unión | `conjunto T = {A, B} union {C};` | A, B, C |
| Intersección | `conjunto C = Nodos inter {B, C, E};` | B, C |
| Diferencia | `conjunto Centro = Nodos diferencia {A, D};` | B, C |
| Por condición | `conjunto Ultimos = {t en Meses: t > 9};` | 10, 11, 12 |
| Pares por condición | `conjunto Salen = {(i,j) en Arcos: i = A};` | (A,B), (A,C) |

Los extremos de un rango pueden ser parámetros ya definidos (`conjunto T = 1..H;`). Para la diferencia también valen `menos`, `minus`, `diff` y `diferença`; para la intersección, `interseccion` e `interseção`; para la unión, `união`. Un conjunto vacío se escribe `{}` (útil cuando los datos llegan desde afuera, sección 5.4). `card(I)` da el número de elementos.

Modelo de prueba con todas las formas:

```iaqsb
conjunto Nodos    = {A, B, C, D};
conjunto Meses    = 1..6;
conjunto Semanas  = {1..4};
conjunto Ciudades = {"San José", "Bogotá", Lima};
conjunto Arcos    = {(A,B), (A,C), (B,C), (B,D), (C,D)};
conjunto Centro   = Nodos diferencia {A, D};
conjunto Todos    = {A, B} union {C, E};
conjunto Comun    = Nodos inter {B, C, E};
conjunto Ultimos  = {t en Meses: t > 3};
conjunto Salen    = {(i,j) en Arcos: i = A};

variable u{Todos} >= 0 <= 1;
variable v{Comun} >= 0 <= 1;
variable w{Centro} >= 0 <= 1;
variable p{Ultimos} >= 0 <= 1;
variable s{Salen} >= 0 <= 1;
variable c{Ciudades} >= 0 <= 1;

maximizar total: suma{i en Todos} u[i] + suma{i en Comun} v[i] + suma{i en Centro} w[i]
               + suma{t en Ultimos} p[t] + suma{(i,j) en Salen} s[i,j]
               + c["San José"] + card(Arcos);
```

El óptimo es 19: 4 + 2 + 2 + 3 + 2 + 1 más los 5 arcos.

## 5. Parámetros y datos

Un parámetro es un dato: un número suelto o una tabla indexada por uno o varios conjuntos. Los parámetros no se resuelven, se leen.

### 5.1 Formas de dar los valores

| Forma | Ejemplo |
| --- | --- |
| Escalar | `parametro tasa = 0.12;` |
| Calculado | `parametro meses = card(T);` |
| Lista en el orden del conjunto | `parametro oferta{Plantas} = [300, 500];` |
| Tabla: una fila por elemento del primer conjunto | `parametro costo{Plantas, Clientes} = [[4, 6, 9], [7, 3, 6]];` |
| Por clave | `parametro dem{Clientes} = {1: 120, 3: 200};` |
| Por clave con pares | `parametro flete{Arcos} = {(Cali,1): 4, (Medellin,2): 3};` |
| Con fórmula de los índices | `parametro cap{t en T} = si t <= 3 entonces 120 sino 140;` |
| Valor por omisión | `parametro peso{Clientes} default 1;` |
| Clave y omisión | `parametro dem{Clientes} = {1: 120} default 0;` |

`=` y `:=` son equivalentes. En una tabla de tres índices se anidan las listas un nivel más (`[[[…], […]], …]`). En un parámetro indexado por un conjunto de pares, la lista sigue el orden de los pares.

Si falta un valor y no hay `default`, el error aparece donde se usa: «falta el dato dem[2] (dalo o pon un valor por defecto: param dem{…} default 0;)».

```iaqsb
conjunto Plantas  = {Cali, Medellin};
conjunto Clientes = 1..3;
conjunto T        = 1..6;
conjunto Arcos    = {(Cali,1), (Cali,2), (Medellin,2), (Medellin,3)};

parametro tasa  = 0.12;
parametro meses = card(T);
parametro oferta{Plantas} = [300, 500];
parametro costo{Plantas, Clientes} = [
  [4, 6, 9],
  [7, 3, 6]
];
parametro flete{Arcos} = {(Cali,1): 4, (Cali,2): 6, (Medellin,2): 3, (Medellin,3): 6};
parametro dem{Clientes} = {1: 120, 3: 200} default 0;
parametro cap{t en T} = si t <= 3 entonces 120 sino 140;
parametro costo_anual{p en Plantas, c en Clientes} = costo[p,c] * (1 + tasa);
parametro peso{Clientes} default 1;

variable x{Plantas, Clientes} >= 0;

minimizar z: suma{p en Plantas, c en Clientes} costo_anual[p,c] * peso[c] * x[p,c];

sujeto a
  of{p en Plantas}:  suma{c en Clientes} x[p,c] <= oferta[p];
  de{c en Clientes}: suma{p en Plantas} x[p,c] >= dem[c];
```

Los parámetros que no se usan (aquí `meses`, `flete`, `cap`) no estorban.

### 5.2 Tablas

La forma de lista anidada corresponde a la tabla que se ve en un libro: la fila es el primer índice y la columna el segundo. Para tablas con muchos ceros o con huecos conviene la forma por clave con `default 0`. Para tablas que vienen de una hoja de cálculo, la forma más cómoda es pasar los datos aparte, en JSON (sección 5.4).

### 5.3 Parámetros calculados

La expresión de un parámetro puede usar otros parámetros, los índices, `si … entonces … sino` (anidable), las operaciones `+ - * / ^` y las funciones de la sección 11. No puede usar variables de decisión.

### 5.4 Datos fuera del texto

Desde la API, el servidor MCP o un archivo `.json`, el modelo se puede acompañar de un objeto `data` cuyos valores reemplazan los del texto. Así el mismo `.iaqsb` sirve para varios casos:

```json
{
  "module": "lp",
  "model": {
    "format": "iaqsb",
    "name": "datos",
    "text": "conjunto I = {}; parametro a{I} default 0; parametro cap = 12; variable x{I} >= 0 <= 10; maximizar f: suma{i en I} a[i] * x[i]; sujeto a c: suma{i en I} x[i] <= cap;",
    "data": {"I": ["p", "q"], "a": {"p": 3, "q": 5}, "cap": 15}
  }
}
```

Reglas de `data`:

- Las claves son los nombres de conjuntos y parámetros; también se aceptan dentro de `"sets"` y `"params"`.
- Un conjunto se da como lista de elementos (`["p", "q"]`) o de tuplas (`[["A","B"], ["A","C"]]`). Debe estar declarado en el texto (con `= {}` si no tiene valor propio).
- Un parámetro se da como número, como listas anidadas en el orden de los conjuntos o como objeto por claves; una clave `"A,B"` cubre dos índices a la vez. Los valores `null` se saltan (toman el `default`) y los textos numéricos con coma decimal (`"3,5"`) se aceptan.

Con este archivo guardado como `modelo.json`, `iaqsb resolver modelo.json` da Z = 65, y `iaqsb barrer modelo.json --dato data.cap --desde 5 --hasta 25 --pasos 4` resuelve el modelo para cada valor de `cap` (sección 16).

## 6. Variables: cotas y tipos

```text
variable nombre{índices} [>= cota_inferior] [<= cota_superior] [tipo];
```

- Sin cotas, la variable es continua y mayor o igual que cero.
- `libre` (`free`, `livre`, `unrestricted`) quita la cota inferior.
- Las cotas pueden ser números, parámetros o expresiones de datos. Para que dependan del índice, el índice debe tener nombre: `variable x{i en I} <= tope[i];`.
- Los atributos se pueden separar con comas: `variable cajas >= 5 <= 50, semientera;`.
- Una variable indexada sobre un dominio con condición solo existe donde la condición se cumple (sección 9).

| Tipo | Palabras | Significado |
| --- | --- | --- |
| Continua | `continua`, `continuo`, `continuous` (o nada) | valor real entre las cotas |
| Entera | `entera`, `entero`, `integer`, `int`, `inteira`, `inteiro` | entero entre las cotas |
| Binaria | `binaria`, `binario`, `binary` | 0 o 1 |
| Semicontinua | `semicontinua`, `semicontinuo`, `semicontinuous` | 0 o un valor entre la cota inferior y la superior |
| Semientera | `semientera`, `semientero`, `semiinteger`, `semiinteira`, `semiinteiro` | 0 o un entero entre las cotas |

```iaqsb
conjunto I = {A, B, C};
parametro tope{I} = [40, 25, 30];

variable x{i en I} >= 0 <= tope[i];
variable n >= 0 <= 10 entera;
variable y{I} binaria;
variable z libre;
variable lote >= 20 <= 100 semicontinua;     # 0, o entre 20 y 100
variable cajas >= 5 <= 50, semientera;       # 0, o un entero entre 5 y 50

maximizar f: suma{i en I} x[i] - 3 * suma{i en I} y[i] + n - z + lote + cajas;

sujeto a
  r1: z >= -5;
  r2: lote + cajas <= 60;
```

El óptimo es 170. Si la cota inferior queda mayor que la superior, el error lo dice: «la variable x tiene la cota inferior (5) mayor que la superior (2)».

**Las cotas importan más que en un modelo lineal corriente**: las restricciones generales de la sección 10 calculan su big-M a partir de ellas.

## 7. Objetivos

```text
maximizar nombre: expresión;
minimizar nombre: expresión;
maximizar expresión;          # sin nombre
Maximizar Z = expresión;      # nombre con «=»
```

Un modelo sin objetivo es válido: se busca una solución factible (el informe lo advierte).

### 7.1 Objetivo cuadrático

En el primer objetivo se admiten productos de dos expresiones lineales y potencias de 2:

```iaqsb
variable x >= 0;
variable y >= 0;
minimizar f: (x - 2)^2 + (y - 1)^2;
sujeto a
  c: x + y <= 2;
```

El óptimo es 0.5 en x = 1.5, y = 0.5. Un ejemplo de cartera de Markowitz con dos activos:

```iaqsb
conjunto A = {Acciones, Bonos};
parametro r{A} = [0.10, 0.04];
variable w{A} >= 0;
minimizar riesgo: 0.04 * w[Acciones]^2 + 0.01 * w[Bonos]^2 + 2 * 0.002 * w[Acciones] * w[Bonos];
sujeto a
  presupuesto: suma{a en A} w[a] = 1;
  retorno: suma{a en A} r[a] * w[a] >= 0.06;
```

Condiciones para el objetivo cuadrático:

- Si es **convexo al minimizar o cóncavo al maximizar** y las variables son **continuas**, lo resuelve HiGHS directamente, con modelos de cualquier tamaño.
- Si **no es convexo** (por ejemplo, `maximizar f: x*y;`) o tiene variables **enteras**, IAQSB lo detecta y usa la búsqueda global del módulo QP-IQP (enumeración de puntos KKT y ramificación y acotamiento), hasta 40 variables; el informe lo dice en una nota y no trae análisis de sensibilidad. Si el modelo es demasiado grande para la búsqueda exhaustiva, el estado es «factible» y el mensaje dice que es un **óptimo local** (mejorado con programación lineal sucesiva desde varios puntos de partida), no el óptimo global. Con más de 40 variables, o para resolverlo con Gurobi o CPLEX, elija ese motor en el botón del motor.
- En las restricciones no se admiten productos de variables: el modelo dejaría de ser lineal.
- Potencias distintas de 1 y 2 de una variable no se admiten.

### 7.2 Varios objetivos

Si hay más de un objetivo, se optimizan **por prioridad, en el orden en que se escriben**: el segundo se optimiza sin empeorar el primero, el tercero sin empeorar los dos anteriores, y así. Cada objetivo puede ser de maximizar o de minimizar, y los adicionales deben ser lineales.

```iaqsb
variable x >= 0 <= 10;
variable y >= 0 <= 10;
maximizar ventas: x + y;      # prioridad 1
maximizar margen: x;          # prioridad 2: solo entre los óptimos de «ventas»
sujeto a
  c: x + y <= 12;
```

La solución es x = 10, y = 2: las ventas valen 12 en cualquier punto con x + y = 12, y entre esos el margen manda llevar x a su cota. El informe trae el valor de cada objetivo. Los modelos con varios objetivos no tienen análisis de sensibilidad (los rangos salen como «-»).

## 8. Restricciones

```text
nombre: expresión relación expresión;
nombre{índices}: expresión relación expresión;     # una restricción por cada índice
a <= expresión <= b;                               # restricción doble (rango)
```

- Relaciones: `<=`, `>=`, `=` (también `=<`, `=>`, `==`, `≤`, `≥`). `<` y `>` se aceptan y se tratan como `<=` y `>=`.
- Las variables pueden estar a ambos lados; IAQSB pasa todo al lado izquierdo.
- En una restricción doble, los extremos deben ser números o datos y los dos signos deben ir en el mismo sentido (`a <= e <= b` o `a >= e >= b`).
- El nombre es opcional. Las restricciones sin nombre se llaman `R1`, `R2`… según su posición entre las instrucciones de restricción.
- Una restricción sin variables se comprueba al leer el modelo: si se cumple, se descarta; si no, es un error.

```iaqsb
conjunto T = 1..4;
parametro d{T} = [100, 120, 90, 150];
variable p{T} >= 0;
variable s{T} >= 0;
minimizar costo: suma{t en T} (10 p[t] + 2 s[t]);
sujeto a
  balance{t en T: t > 1}: s[t-1] + p[t] - s[t] = d[t];
  inicio: p[1] - s[1] = d[1];
  rango{t en T}: 80 <= p[t] <= 130;          # restricción doble
  p[1] + p[2] >= 200;                         # sin nombre: se llama R4
  s.t. final: s[4] >= 0;                      # «s.t.» delante de una restricción
```

El óptimo es 4640. Los índices admiten aritmética: `s[t-1]` con `t` numérico.

## 9. Sumatorias y condiciones

### 9.1 Dominios de índices

Un dominio de índices va entre llaves y se usa igual en `suma`, en parámetros, variables y restricciones indexadas:

| Forma | Significado |
| --- | --- |
| `{i en I}` | i recorre I |
| `{i en I, j en J}` | todas las combinaciones |
| `{(i,j) en Arcos}` | recorre un conjunto de pares |
| `{I, J}` | sin nombre de índice (como en AMPL); vale cuando el índice no se necesita en la expresión |
| `{i en I: condición}` o `{i en I \| condición}` | solo los índices que cumplen la condición |
| `{t en 1..T}` | un rango escrito en el momento |

`en` se escribe también `in`, `em` o `∈`.

### 9.2 Sumatorias

`suma{dominio} término` suma el término para cada índice (también `sum`, `soma`, `sumatoria`, `somatorio`, `Σ`, `∑`). El cuerpo de la suma es **un término**: un producto o cociente, no una suma. Por eso

```text
suma{i en I} c[i] * x[i] + 5        # = (Σ c[i] x[i]) + 5
suma{i en I} (x[i] + y[i])          # los paréntesis meten las dos variables en la suma
```

Las sumas se pueden anidar (`suma{i en I} suma{j en J: j > i} x[i,j]`) y multiplicar por un número (`2 suma{i en I} x[i]`).

### 9.3 Condiciones

Dentro de un dominio o de un `si … entonces … sino` se pueden usar:

- comparaciones `=`, `!=` (o `<>`, `≠`), `<`, `<=`, `>`, `>=`; entre nombres solo `=` y `!=`;
- pertenencia: `i en Vip`, `(i,j) en Arcos`;
- conectores `y` / `and` / `e` / `&&`, `o` / `or` / `ou` / `||`, `no` / `not` / `nao` / `!`, y paréntesis.

Las condiciones se evalúan con los datos, no con las variables de decisión.

Ruta más corta de A a D como flujo en una red, con sumas sobre pares filtradas por condición:

```iaqsb
conjunto N = {A, B, C, D};
conjunto Arcos = {(A,B), (A,C), (B,C), (B,D), (C,D)};
parametro long{Arcos} = {(A,B): 4, (A,C): 2, (B,C): 1, (B,D): 5, (C,D): 8};
variable x{Arcos} >= 0 <= 1;
minimizar distancia: suma{(i,j) en Arcos} long[i,j] * x[i,j];
sujeto a
  flujo{n en N}:
    suma{(i,j) en Arcos: i = n} x[i,j] - suma{(i,j) en Arcos: j = n} x[i,j]
      = si n = A entonces 1 sino si n = D entonces -1 sino 0;
```

La distancia mínima es 9, por A → B → D (4 + 5).

Sumas con varias condiciones, `Σ`, `∈`, la barra `|` y multiplicación implícita:

```iaqsb
conjunto I = 1..4;
conjunto J = 1..4;
parametro c{i en I, j en J} = abs(i - j) + 1;
variable x{I, J} >= 0 <= 10;
minimizar z: Σ{i ∈ I, j ∈ J: i != j} c[i,j] * x[i,j];
sujeto a
  fila{i en I}: suma{j en J: j != i y c[i,j] <= 3} x[i,j] >= 2;
  col{j en J}:  suma{i en I | i < j o i > j + 1} x[i,j] <= 15;
  diag: 2 suma{i en I} x[i,i] + 3(x[1,2] + x[2,1]) >= 4;
  tri:  suma{i en I} suma{j en J: j > i} x[i,j] <= 30;
```

### 9.4 Variables sobre dominios con condición

Las variables pueden declararse solo donde tienen sentido. Aquí `x` existe para los arcos y para los pares que salen de un nodo especial; usar `x[A,C]` daría el error «la variable x[A,C] no existe (no está en sus conjuntos)»:

```iaqsb
conjunto N = {A, B, C};
conjunto Arcos = {(A,B), (B,C)};
conjunto Vip = {B};
variable x{i en N, j en N: (i,j) en Arcos o i en Vip} >= 0 <= 1;
maximizar z: suma{i en N, j en N: (i,j) en Arcos o i en Vip} x[i,j];
```

Se crean `x[A,B]`, `x[B,A]`, `x[B,B]` y `x[B,C]`.

## 10. Restricciones generales

IAQSB acepta construcciones que no son lineales y las **reformula solo** como restricciones lineales con variables auxiliares (binarias cuando hace falta). El big-M de cada reformulación **se calcula con las cotas de las variables que intervienen**: por eso esas variables necesitan cotas finitas. Si no las tienen, el error lo dice: «para abs( ) hacen falta cotas finitas en las variables que intervienen (p. ej. var x <= 100;) o un big-M general: option bigM = 10000;».

La revisión del modelo (`iaqsb revisar`, el panel de tamaño del editor) lista cada reformulación con el número de variables y restricciones nuevas. Las variables auxiliares aparecen en la lista general con nombres que empiezan por `_` (`_abs1`, `_maxb3`, `_o5`…), pero no en las tablas por índices.

### 10.1 Valor absoluto: `abs(e)`

En el objetivo o en una restricción. Ejemplo: acercar la producción total a una meta de 100 sin pasar de la capacidad.

```iaqsb
variable x >= 0 <= 100;
variable y >= 0 <= 100;
minimizar desvio: abs(x + y - 100);
sujeto a
  capacidad: 2x + y <= 90;
  minimo_y: y >= 50;
```

Desviación mínima: 10. En una restricción: `cerca: abs(x - 10) <= 3;` deja x entre 7 y 13.

```iaqsb
variable x >= 0 <= 20;
maximizar z: x;
sujeto a
  cerca: abs(x - 10) <= 3;
```

*Linealización:* si las cotas L y U de la expresión tienen el mismo signo, `abs` se reemplaza por la expresión o su negativo, sin binarias. Si no, se agrega t ≥ 0 y una binaria b que indica el signo, con 6 restricciones y big-M tomados de L y U.

### 10.2 Máximo y mínimo: `max(e1, e2, …)`, `min(e1, e2, …)`

Repartir 90 horas entre tres máquinas para minimizar la carga de la más ocupada:

```iaqsb
conjunto M = 1..3;
parametro vel{M} = [1, 1.5, 2];
variable h{M} >= 0 <= 60;
minimizar pico: max(h[1], h[2], h[3]);
sujeto a
  produccion: suma{m en M} vel[m] * h[m] >= 135;
  menor: min(h[1], h[2], h[3]) >= 5;
```

El pico mínimo es 30. *Linealización:* una variable y y una binaria por argumento, que elige cuál alcanza el máximo (2n + 1 restricciones); el big-M de cada argumento es la cota superior más alta menos la cota inferior de ese argumento. `min` es `max` con el signo cambiado. Con un solo argumento no se agrega nada.

Una restricción que empieza por `max(` o `min(` necesita nombre (`pico: max(…) <= 50;`), porque al principio de una instrucción `max` y `min` abren un objetivo.

### 10.3 Lineal por tramos: `pwl(x; (x0, f0), (x1, f1), …)`

La función se da por sus puntos (x, f(x)), con x creciente; sirve para costos con descuentos por cantidad o economías de escala. También se escribe `tramos(…)`.

```iaqsb
variable q >= 0 <= 300;
minimizar costo: pwl(q; (0, 0), (100, 500), (300, 1100));
sujeto a
  demanda: q >= 150;
```

Costo mínimo: 650 (100 unidades a 5 y 50 a 3). *Linealización:* formulación con pesos λ por punto y una binaria por tramo (solo dos puntos vecinos pueden ser positivos); no usa big-M, pero **la expresión queda obligada a estar entre el primer y el último x**.

### 10.4 Funciones lógicas: `and`, `or`, `not`

Sus argumentos deben ser variables binarias (o `1 − binaria`). `and` se escribe también `y` o `e`; `or`, `o` u `ou`; `not`, `no` o `nao`.

```iaqsb
var a binary; var b binary; var z binary;
maximize f: z - 0.5*a - 0.4*b;
subject to
  c: z = and(a, b);
```

```iaqsb
variable a binaria; variable b binaria; variable c binaria;
variable u binaria; variable v binaria;
maximizar f: 2u + v - a - b - c;
sujeto a
  r1: u <= or(a, b);
  r2: v <= not(c);
  r3: u + v <= 1 + y(a, c);
```

*Linealización:* `and` y `or` de n binarias agregan una binaria y n + 1 restricciones (sin big-M); `not(b)` es `1 − b`.

### 10.5 Restricciones indicadoras: `si b = v entonces relación`

La relación se exige solo cuando la binaria toma el valor indicado (0 o 1). Hay tres escrituras equivalentes: `si y[i] = 0 entonces …`, `if y[i] = 1 then …` y `y[i] = 1 -> …` (también `→`, `⇒`, `si (y[i] = 1) entonces …`). Ejemplo de costos fijos:

```iaqsb
conjunto P = {A, B, C};
parametro fijo{P} = [500, 300, 400];
parametro cu{P}   = [2, 5, 3];
variable abre{P} binaria;
variable q{P} >= 0 <= 100;
minimizar costo: suma{p en P} (fijo[p] * abre[p] + cu[p] * q[p]);
sujeto a
  demanda: suma{p en P} q[p] >= 150;
  cerrada{p en P}: si abre[p] = 0 entonces q[p] <= 0;
  piso{p en P}: abre[p] = 1 -> q[p] >= 30;
  dos: atmost(2, abre[A], abre[B], abre[C]);
```

Costo mínimo: 1250. *Linealización:* `e <= hi` se convierte en `e <= hi + (U − hi)(1 − s)`, donde U es la cota superior de e calculada con las cotas de sus variables y s es la binaria (o 1 − binaria si la condición es `= 0`); `>=` es análogo con la cota inferior; `=` produce las dos.

### 10.6 Disyunciones: `relación1 o relación2 …`

Se cumple al menos una de las relaciones. Secuenciar dos tareas en una máquina (una antes que la otra):

```iaqsb
variable s1 >= 0 <= 100;
variable s2 >= 0 <= 100;
variable fin >= 0 <= 200;
minimizar m: fin;
sujeto a
  orden: s1 + 5 <= s2 o s2 + 3 <= s1;
  f1: fin >= s1 + 5;
  f2: fin >= s2 + 3;
```

El fin mínimo es 8. El separador puede ser `o`, `or`, `ou` o `||`; **si la relación anterior termina en un número, use `or`, `ou` o `||`** (sección 14). *Linealización:* una binaria por relación, cada relación como indicadora de su binaria, y la suma de binarias mayor o igual que 1.

### 10.7 Cardinalidad e implicación

| Escritura | Significado | Restricción que se agrega |
| --- | --- | --- |
| `atmost(k, b1, b2, …)` (`alomas`, `nomasde`, `nomaximo`) | a lo sumo k | b1 + b2 + … ≤ k |
| `atleast(k, b1, b2, …)` (`almenos`, `pelomenos`) | al menos k | b1 + b2 + … ≥ k |
| `exactly(k, b1, b2, …)` (`exactamente`, `exatamente`) | exactamente k | b1 + b2 + … = k |
| `implies(a, b)` (`implica`) | si a = 1 entonces b = 1 | a − b ≤ 0 |

Son restricciones lineales directas, sin variables auxiliares. Un argumento puede ser una suma (`exactly(3, suma{k en K} y[k])`). `implies` exige binarias; en `atmost`, `atleast` y `exactly` no se comprueba el tipo, así que con variables continuas limitan su suma.

```iaqsb
conjunto Proyectos = 1..5;
parametro valor{Proyectos} = [12, 9, 7, 6, 4];
parametro costo{Proyectos} = [6, 5, 4, 3, 2];
variable elegir{Proyectos} binaria;
maximizar total: suma{k en Proyectos} valor[k] * elegir[k];
sujeto a
  presupuesto: suma{k en Proyectos} costo[k] * elegir[k] <= 12;
  al_menos_dos: atleast(2, elegir[3], elegir[4], elegir[5]);
  tres: exactly(3, suma{k en Proyectos} elegir[k]);
  dependencia: implies(elegir[1], elegir[5]);
```

Valor máximo: 23 (proyectos 1, 3 y 5). Las mismas construcciones con los nombres en español:

```iaqsb
conjunto K = 1..4;
variable y{K} binaria;
variable x{K} >= 0 <= 10;
maximizar f: suma{k en K} x[k] - suma{k en K} 3 y[k];
sujeto a
  a1: alomas(2, y[1], y[2], y[3], y[4]);
  a2: almenos(1, y[1], y[2]);
  a3: exactamente(1, y[3], y[4]);
  a4: implica(y[4], y[1]);
  i1{k en K}: si (y[k] = 0) entonces x[k] <= 0;
  i2: if y[2] = 1 then x[2] >= 2;
  i3: y[3] == 0 -> x[3] = 0;
  d1: x[1] <= 3 || x[1] >= 8;
```

### 10.8 Big-M general: `opcion bigM = …`

Cuando alguna variable no tiene cota finita, se puede dar un big-M general (`option bigM`, `opcion bigM`, `opcao bigM`; también `big_m`). Se usa **solo en las cotas que faltan**; las que existen se siguen usando.

```iaqsb
opcion bigM = 1000;
variable x libre;
variable y libre;
minimizar f: abs(x - 3) + abs(y + 2);
sujeto a
  c: x + y >= 4;
```

Un big-M muy grande debilita la relajación lineal y puede causar problemas numéricos. Lo recomendable es dar cotas ajustadas a cada variable; `bigM` es el último recurso. `bigM` es la única opción que existe: cualquier otra da «opción desconocida».

## 11. Funciones de datos

Con argumentos que son datos (no variables de decisión) se calculan en el momento:

| Función | También | Resultado |
| --- | --- | --- |
| `abs(a)` | | valor absoluto |
| `sqrt(a)` | `raiz` | raíz cuadrada |
| `exp(a)` | | exponencial |
| `log(a)` | `ln` | logaritmo natural |
| `log10(a)` | | logaritmo en base 10 |
| `floor(a)` | `piso` | parte entera por debajo |
| `ceil(a)` | `techo`, `teto` | parte entera por encima |
| `round(a)` | `redondear`, `arredondar` | redondeo |
| `min(a, b, …)`, `max(a, b, …)` | `minimo`, `maximo` | menor y mayor |
| `card(I)` | `tamano`, `tamanho` | número de elementos del conjunto |
| `a ^ b` | | potencia |

No existe operador de residuo (`mod`); se escribe `a - b * floor(a / b)`. Con variables de decisión solo `abs`, `min`, `max`, `and`, `or`, `not` y `pwl` tienen sentido (sección 10); las demás dan el error «la función sqrt no admite variables de decisión».

```iaqsb
conjunto I = 1..5;
parametro a = max(3, 7, 5);
parametro b = raiz(16) + piso(2.7) + techo(2.1) + redondear(2.5);
parametro c = log(exp(2)) + log10(100) + abs(-1) + 2^3;
parametro n = tamano(I);
variable q >= 0 <= 300;
minimizar costo: tramos(q; (0, 0), (100, 500), (300, 1100));
sujeto a
  dem: q >= a + b + c + n * 10;
```

## 12. Palabras clave en español, inglés y portugués

Esta es la lista completa que acepta el lector (`kw()` en `parse.rs`). No distingue mayúsculas ni tildes, así que `parámetro`, `restricción`, `não`, `então` o `união` también valen.

| Concepto | Español | Inglés | Portugués |
| --- | --- | --- | --- |
| Conjunto | `conjunto`, `conj` | `set` | `conjunto` |
| Parámetro | `parametro`, `parametros` | `param` | `parametro` |
| Variable | `variable`, `variables` | `var` | `variavel`, `variaveis` |
| Maximizar | `maximizar`, `maximice`, `maximo`, `max` | `maximize`, `maximise`, `max` | `maximizar` |
| Minimizar | `minimizar`, `minimice`, `minimo`, `min` | `minimize`, `minimise`, `min` | `minimizar` |
| Restricciones | `sujeto a`, `sujeta a`, `s.a.`, `sa`, `restriccion`, `restricciones` | `subject to`, `s.t.`, `st`, `constraint`, `constraints` | `sujeito a`, `restricao`, `restricoes` |
| Suma | `suma`, `sumatoria`, `Σ`, `∑` | `sum` | `soma`, `somatorio` |
| Pertenencia | `en`, `∈` | `in` | `em` |
| Y | `y`, `&&` | `and` | `e` |
| O | `o`, `\|\|` | `or` | `ou` |
| No | `no`, `!` | `not` | `nao` |
| Si … entonces … sino | `si`, `entonces`, `sino` | `if`, `then`, `else` | `se`, `entao`, `senao` |
| Binaria | `binaria`, `binario` | `binary` | `binaria` |
| Entera | `entera`, `entero` | `integer`, `int` | `inteira`, `inteiro` |
| Continua | `continua`, `continuo` | `continuous` | `continua` |
| Libre | `libre` | `free`, `unrestricted` | `livre` |
| Semicontinua | `semicontinua`, `semicontinuo` | `semicontinuous` | `semicontinua` |
| Semientera | `semientera`, `semientero` | `semiinteger` | `semiinteira`, `semiinteiro` |
| Valor por omisión | `default`, `defecto`, `omision` | `default` | `padrao` |
| Unión | `union` | `union` | `uniao` |
| Intersección | `inter`, `interseccion` | `inter` | `intersecao` |
| Diferencia | `diferencia`, `menos` | `diff`, `minus` | `diferenca` |
| Opción | `opcion` | `option` | `opcao` |
| Cardinalidad | `alomas`, `nomasde`, `nomaximo`, `almenos`, `exactamente`, `implica` | `atmost`, `atleast`, `exactly`, `implies` | `pelomenos`, `exatamente` |

Otros signos: `->`, `→` y `⇒` en las indicadoras; `≤`, `≥`, `≠`; `·` y `×` para multiplicar; `−` (signo menos tipográfico).

**Palabras reservadas.** Estas palabras no deben usarse como nombres de conjuntos, parámetros, variables ni restricciones. Los casos que más fallan son una restricción llamada `minimo`, `maximo`, `max` o `min` (se lee como un objetivo), una variable llamada `suma` o `en`, y nombres de una letra como `o`. La letra `y` funciona como variable en la práctica (el modo rápido la usa), pero conviene evitarla en modelos largos.

## 13. Comentarios, números y textos

- Comentarios: `# …` y `// …` hasta el final de la línea; `/* … */` en varias líneas.
- Números: con **punto decimal** (`0.12`, `.5`) y notación científica (`1e4`, `2.5E-3`). La coma separa elementos de listas y argumentos, nunca decimales, aunque la aplicación esté configurada con coma decimal.
- Textos: entre comillas `"…"`, `'…'`, `«…»` o `“…”`. Sirven para elementos con espacios, tildes o signos (`"San José"`) y se usan igual como índice: `c["San José"]`.
- Un nombre suelto como índice se toma como elemento: `x[Bogota]`. Si coincide con un índice de un dominio o con un parámetro escalar, se usa ese valor.
- Multiplicación implícita: un número seguido de un nombre, de `suma` o de un paréntesis (`3x`, `10 p[t]`, `2 suma{…}`, `3(x + y)`).
- Los nombres empiezan con letra o `_` y pueden llevar letras (con tildes), dígitos, `_` y `'`.

## 14. Errores frecuentes

Cada error trae la línea y la columna. Los más comunes:

| Síntoma (mensaje) | Causa | Solución |
| --- | --- | --- |
| `se esperaba «;» al final de la instrucción y hay «max»` | falta el `;` de la línea anterior | terminar cada instrucción con `;` |
| `«w» no está definido (¿falta declararlo con param, var o set?)` | errata, o se declaró alguna variable y se apagó el modo rápido | declarar el nombre o corregirlo |
| `la variable y[i] no existe (no está en sus conjuntos)` después de `suma{i en I} x[i] + y[i]` | el cuerpo de la suma es solo `x[i]` | `suma{i en I} (x[i] + y[i])` |
| `la variable x[c] no existe` | el índice no es un elemento del conjunto, o la variable se declaró con condición | revisar el conjunto o la condición |
| `falta el dato tope[i]` en `variable x{I} <= tope[i]` | el índice no tiene nombre | `variable x{i en I} <= tope[i]` |
| `falta el dato c[A] (dalo o pon un valor por defecto…)` | parámetro sin valor para ese índice | dar el valor o `default 0` |
| `c: la lista tiene 3 valores y el conjunto 2 elementos` | lista más larga o más corta que el conjunto | revisar el orden y el tamaño |
| `c[C] no corresponde a los conjuntos de c` | clave que no está en el conjunto | corregir la clave |
| `«x» ya está definido` | dos declaraciones con el mismo nombre (aunque sean de distinta clase) | renombrar |
| `el conjunto I tiene elementos repetidos` | elemento duplicado | quitarlo |
| `producto de variables de decisión: el modelo no es lineal…` | `x * y` en una restricción | reformular (por ejemplo con binarias e indicadoras) |
| `potencia de una variable de decisión` / `no se puede dividir entre una variable` | `x^3`, `1/x` | el lenguaje es lineal (y cuadrático solo en el objetivo) |
| `para abs( ) hacen falta cotas finitas…` | restricción general con variables sin cota | poner cotas o `opcion bigM = …` |
| `las funciones lógicas (and, or, not) y las indicadoras van con variables binarias` | la condición de `si … entonces` o un `and` usa una variable no binaria | declararla `binaria` |
| `la restricción c no tiene variables y no se cumple` | relación entre números que es falsa | revisar los datos |
| `se esperaba un número, un nombre o «(» y hay «{»` en una línea que empieza por `minimo{…}:` o `max{…}:` | el nombre de la restricción es una palabra reservada | renombrar (sección 12) |
| `una restricción tiene a lo sumo dos relaciones (a <= x <= b)` | `a <= x <= b <= c` | partirla en dos |
| `opción desconocida: foo (la que hay es bigM)` | opción que no existe | solo `bigM` |
| `el modelo no tiene variables (decláralas con var x{…};)` | modelo solo con datos | declarar variables o escribir el objetivo |

Sobre el objetivo cuadrático: si no es convexo o tiene variables enteras, el informe trae la nota «se usó la búsqueda global del módulo QP-IQP» (sección 7). Si además dice **óptimo local**, el modelo era demasiado grande para la búsqueda exhaustiva: pruebe a reformularlo o use Gurobi o CPLEX.

## 15. Resultados: tablas por índices

Además del informe habitual de LP-ILP (valor, costo reducido y rangos de cada variable; lado izquierdo, holgura, precio sombra y rangos de cada restricción), un modelo algebraico trae **una tabla por familia**:

- **Variables de dos índices** (`x[i,j]`): tabla de doble entrada, con el primer conjunto en las filas, el segundo en las columnas y totales por fila y por columna (hasta 40 columnas; con más, se muestra como lista).
- **Variables de uno, tres o más índices**: lista con una columna por índice, el valor y el costo reducido. Con más de 40 combinaciones se muestran solo las distintas de cero, y la tabla lo indica («Solo las distintas de cero (12 de 600)»).
- **Restricciones indexadas**: lista con los índices, el lado izquierdo y el precio sombra. Solo aparece cuando hay precios sombra, es decir, en modelos continuos con un solo objetivo.

Por ejemplo, el transporte de la sección 5.1 produce:

```text
Variable x[Plantas, Clientes]
Plantas \ Clientes    1  2    3  Total
------------------  ---  -  ---  -----
              Cali  120  0    0    120
          Medellin    0  0  200    200
             Total  120  0  200    320
```

En la lista general las variables se llaman `x[Cali,1]`, `x[Medellin,3]`, y las restricciones `of[Cali]`, `de[2]`. El informe también trae el tamaño del modelo armado (variables, enteras, restricciones, coeficientes distintos de cero, conjuntos y familias), las notas de reformulación y, si hay varios objetivos, el valor de cada uno.

## 16. Uso desde la línea de comandos

El programa `iaqsb` reconoce como modelo algebraico los archivos con extensión `.iaqsb` o `.ml` (`.MOD` es el formato del módulo de selección del modo de transporte). Las opciones globales `--json` (salida JSON), `--md` (Markdown breve para agentes de IA) y `--lang en|pt` (idioma de la salida) valen para todas las órdenes.

| Tarea | Orden |
| --- | --- |
| Revisar sin resolver (errores con línea y columna, o tamaño y reformulaciones) | `iaqsb revisar modelo.iaqsb` |
| Resolver | `iaqsb resolver modelo.iaqsb` |
| Resolver con otro motor y parámetros | `iaqsb resolver modelo.iaqsb --motor gurobi --tiempo 60 --brecha 0.01 --hilos 4 --metodo barrier --registro` |
| Informe para entregar (Word, PDF, HTML, Excel o Markdown, según la extensión) | `iaqsb informe modelo.iaqsb -s informe.docx --autor "…" --curso "…" --institucion "…" --coma` |
| Diagnóstico de infactibilidad (IIS y relajación) | `iaqsb diagnosticar modelo.iaqsb` |
| Análisis paramétrico exacto de un lado derecho | `iaqsb parametrico modelo.iaqsb --lado maquina --desde 50 --hasta 200` |
| Análisis paramétrico de un costo | `iaqsb parametrico modelo.iaqsb --costo 'x[A]' --desde 0 --hasta 10` |
| Ajuste automático de los parámetros del motor | `iaqsb ajustar modelo.iaqsb --tiempo 20` |
| Exportar el modelo expandido | `iaqsb convertir modelo.iaqsb modelo.mps` (también `.lp`, `.LPP`, `.json`, `.xlsx`) |
| Motores disponibles en el equipo | `iaqsb motores` |

Ejemplos con el modelo de mezcla de la sección 3, guardado como `mezcla.iaqsb`:

```text
$ iaqsb revisar mezcla.iaqsb
Modelo correcto: 3 variables (0 enteras), 4 restricciones, 6 coeficientes distintos de cero.

$ iaqsb parametrico mezcla.iaqsb --lado maquina --desde 50 --hasta 200
Desde  Hasta   Estado  Pendiente  Z al inicio  Z al final
-----  -----  -------  ---------  -----------  ----------
   50    100  optimal        2.5          145         270
  100    160  optimal     1.3333          270         350
  160    200  optimal          0          350         350
```

En `--costo` y `--lado` se usa el nombre expandido (`x[A]`, `mercado[B]`) o la posición, empezando en 0. En la terminal, los corchetes van entre comillas.

**Barrer un dato («¿qué pasa si…?»).** `iaqsb barrer` cambia un dato del modelo JSON, así que un `.iaqsb` suelto no tiene datos que barrer. Se envuelve en un `.json` con `data` (sección 5.4) y se barre `data.<nombre>`:

```text
$ iaqsb barrer modelo.json --dato data.cap --desde 5 --hasta 25 --pasos 4
Valor   Estado  Métrica                               Resultado
-----  -------  -------  --------------------------------------
    5  optimal       25              Máximo Z = 25 en x[q] = 5.
   10  optimal       50             Máximo Z = 50 en x[q] = 10.
   15  optimal       65   Máximo Z = 65 en x[p] = 5, x[q] = 10.
   20  optimal       80  Máximo Z = 80 en x[p] = 10, x[q] = 10.
   25  optimal       80  Máximo Z = 80 en x[p] = 10, x[q] = 10.
```

**Por lotes.** Como cada orden devuelve un código de salida distinto de cero si hay error y acepta `--json`, se puede revisar y resolver una carpeta entera:

```sh
for f in modelos/*.iaqsb; do
  iaqsb revisar "$f" && iaqsb --json resolver "$f" > "resultados/$(basename "$f" .iaqsb).json"
done
```

`iaqsb grafico modelo.iaqsb` da los datos del método gráfico de un modelo de dos variables (también lee `.LPP`, `.mps`, `.lp` y `.json`).

## 17. Uso desde MCP y la API JSON

El servidor `iaqsb-mcp` y la API (`iaqsb --json llamar <operación>`) aceptan el texto del lenguaje como modelo del módulo `lp`:

```json
{"module": "lp", "model": {"format": "iaqsb", "text": "set I = {A, B}; …", "name": "mi_modelo", "data": {}}}
```

`format` admite también `ml`, `algebra`, `algebraico`, `modelo`, `lenguaje`, `linguagem` y `ampl`; `name` y `data` son opcionales.

| Herramienta MCP | Uso con el lenguaje |
| --- | --- |
| `check_model` | `{"text": "…", "data": {…}}`: revisa sin resolver; devuelve los errores con línea y columna o el tamaño y las reformulaciones |
| `solve` | `{"module": "lp", "model": {"format": "iaqsb", "text": "…"}, "options": {…}}`; los parámetros del motor (`time_limit`, `mip_gap`, `threads`, `solver`…) van en `options` |
| `diagnose_infeasibility` | `{"model": {"format": "iaqsb", "text": "…"}}` |
| `convert_model` | pasa el modelo expandido a `lp`, `mps`, `lpp` o `json`, y una hoja (JSON, LP o MPS) al lenguaje con `"to": "iaqsb"` |
| `what_if` | barre un dato de `data` con `"path": "/data/cap"` |
| `parametric_analysis`, `tune_parameters` | igual que con cualquier modelo de LP-ILP |
| `export_report` | informe en `docx`, `pdf`, `html`, `xlsx` o `md` |

El valor `"iaqsb"` de `convert_model` funciona aunque el esquema de la herramienta solo enumere `lp`, `mps`, `lpp` y `json`; un cliente que valide estrictamente el esquema puede rechazarlo, y en ese caso se usa la API (`convert` con `"to": "iaqsb"`).

Un agente de IA puede seguir este flujo: escribir el modelo, llamar a `check_model` hasta que no haya errores, resolver con `solve`, pedir partes del informe con `get_report_part` y exportar con `export_report`. Las tablas por índices llegan en el Markdown del informe.

## 18. Conversión desde la hoja y desde WinQSB

**Desde la aplicación.** En el módulo LP-ILP, el botón **Desde la hoja** escribe la hoja abierta (incluido un `.LPP` de WinQSB, un `.LP` o un `.MPS` recién abiertos) como modelo algebraico sin índices: una declaración por variable con sus cotas y tipo, el objetivo `z` y una restricción por fila. **Pasar a la hoja** hace lo contrario: expande los índices (pide confirmar si pasa de 2000 variables). Al guardar desde el modo algebraico se elige el formato: el texto `.iaqsb` o el modelo expandido en MPS, LP o `.LPP` de WinQSB.

**Desde la API o MCP.** La operación `convert` con `"to": "iaqsb"` hace lo mismo que **Desde la hoja**. Por ejemplo, con el archivo de ejemplo `LP.LP` de WinQSB:

```sh
iaqsb importar LP.LP > hoja.json
iaqsb --json llamar convert "{\"model\": $(python3 -c 'import json;print(json.dumps(json.load(open("hoja.json"))["model"]))'), \"to\": \"iaqsb\"}" \
  | python3 -c 'import json,sys;print(json.load(sys.stdin)["result"]["content"])' > LP.iaqsb
```

produce `LP.iaqsb`:

```iaqsb
# LP Sample Problem
# Escrito por IAQSB a partir de la hoja.

var X1;
var X2;

maximizar z: 50*X1 + 60*X2;

sujeto a
C1: 2*X1 + 3*X2 <= 180;
C2: 3*X1 + 2*X2 <= 150;
```

que da el mismo óptimo que el archivo original (3780). Los nombres con espacios o signos se cambian por `_`, y los que no empiezan con una letra o `_` llevan el prefijo `v_`.

Desde la línea de comandos: `iaqsb convertir hoja.LPP modelo.iaqsb`.

Los nombres que coinciden con palabras reservadas o con funciones del lenguaje (una variable llamada `Suma` o `en`, una restricción llamada `max`) se escriben con un `_` al final (`Suma_`, `en_`, `max_`), y si dos nombres quedan iguales al limpiarlos (`x 1` y `x_1`) el segundo lleva `_2`; así el texto siempre compila.
