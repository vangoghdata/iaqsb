// Ejemplos del lenguaje algebraico. Todos son originales de IAQSB (datos
// ilustrativos, no tomados de libros) y cada uno se resuelve tal cual; los
// comentarios van en español e inglés.

export interface EjemploAlgebra {
  id: string;
  /** Título (se pasa por t() al mostrarlo). */
  titulo: string;
  /** Qué muestra, en una línea (se pasa por t()). */
  resumen: string;
  texto: string;
}

export const EJEMPLOS_ALGEBRA: EjemploAlgebra[] = [
  {
    id: 'transporte',
    titulo: 'Transporte con índices',
    resumen: 'Plantas × ciudades: x[i,j] en tabla de doble entrada.',
    texto: `# Transporte con índices: 3 plantas abastecen a 5 ciudades al menor costo.
# Transportation with indices: 3 plants supply 5 cities at minimum cost.
# Original de IAQSB (datos ilustrativos / illustrative data).

conjunto Plantas  = {Cali, Medellin, Bogota};
conjunto Ciudades = {Pasto, Pereira, Tunja, Neiva, Ibague};

parametro oferta{Plantas}   = [300, 500, 400];        # unidades por semana / units per week
parametro demanda{Ciudades} = [180, 220, 160, 240, 200];

# costo por unidad enviada (fila = planta, columna = ciudad) / unit shipping cost
parametro costo{Plantas, Ciudades} = [
  [4, 6, 9, 5, 7],
  [7, 3, 6, 6, 4],
  [9, 5, 2, 4, 5]
];

variable x{Plantas, Ciudades} >= 0;    # unidades enviadas / units shipped

minimizar costo_total: suma{i en Plantas, j en Ciudades} costo[i,j] * x[i,j];

sujeto a
  capacidad{i en Plantas}: suma{j en Ciudades} x[i,j] <= oferta[i];
  atender{j en Ciudades}:  suma{i en Plantas} x[i,j] >= demanda[j];
`
  },
  {
    id: 'produccion',
    titulo: 'Producción por periodos',
    resumen: 'Balance de inventario mes a mes (palabras clave en inglés).',
    texto: `# Planeación de la producción por periodos con balance de inventario.
# Multi-period production planning with inventory balance.
# Original de IAQSB (datos ilustrativos / illustrative data).

set Products = {Mesa, Silla};
set T = 1..6;                                  # meses / months

param demand{Products, T} = [
  [40, 60, 80, 70, 50, 90],
  [120, 150, 200, 180, 140, 210]
];
param hours{Products} = [3, 1];                # horas por unidad / hours per unit
param capacity{t in T} = if t <= 3 then 420 else 480;   # horas por mes / hours per month
param cost{Products}  = [55, 18];              # costo de producir / production cost
param hold{Products}  = [4, 1.5];              # costo de inventario por mes / holding cost
param s0{Products}    = [10, 30];              # inventario inicial / initial stock

var make{Products, T} >= 0;                    # unidades producidas / units made
var stock{Products, T} >= 0;                   # inventario al final del mes / ending stock

minimize total_cost:
  sum{p in Products, t in T} (cost[p] * make[p,t] + hold[p] * stock[p,t]);

subject to
  # inventario anterior + producción − demanda = inventario final
  # previous stock + production − demand = ending stock
  balance{p in Products, t in T: t > 1}: stock[p,t-1] + make[p,t] - stock[p,t] = demand[p,t];
  first{p in Products}: s0[p] + make[p,1] - stock[p,1] = demand[p,1];
  hours_cap{t in T}: sum{p in Products} hours[p] * make[p,t] <= capacity[t];
`
  },
  {
    id: 'localizacion',
    titulo: 'Localización con binarias',
    resumen: 'Abrir centros: costos fijos e indicadoras «si … entonces» y «->».',
    texto: `# Localización de centros de distribución con binarias e indicadoras.
# Facility location with binary variables and indicator constraints.
# Original de IAQSB (datos ilustrativos / illustrative data).

conjunto Sitios   = {Bogota, Medellin, Cali, Barranquilla};
conjunto Clientes = 1..6;

parametro fijo{Sitios} = [9000, 7500, 6800, 7000];   # costo fijo de abrir / fixed cost
parametro cap{Sitios}  = [700, 500, 450, 500];       # capacidad / capacity
parametro dem{Clientes} = [150, 180, 120, 200, 90, 160];
parametro c{Sitios, Clientes} = [
  [ 4,  6,  9, 12,  7,  5],
  [ 6,  3,  5, 10,  9,  8],
  [10,  7,  3,  6, 11,  9],
  [12, 11, 10,  4,  3,  7]
];

variable abrir{Sitios} binaria;                    # 1 si se abre el centro / 1 if opened
variable x{Sitios, Clientes} >= 0 <= 1000;         # unidades enviadas / units shipped

minimizar costo: suma{i en Sitios} fijo[i] * abrir[i]
               + suma{i en Sitios, j en Clientes} c[i,j] * x[i,j];

sujeto a
  demanda{j en Clientes}: suma{i en Sitios} x[i,j] >= dem[j];
  capacidad{i en Sitios}: suma{j en Clientes} x[i,j] <= cap[i];
  # indicadora: si el centro está cerrado no despacha nada
  # indicator: a closed site ships nothing
  cerrado{i en Sitios}: si abrir[i] = 0 entonces suma{j en Clientes} x[i,j] <= 0;
  # indicadora (otra sintaxis): si se abre, despacha al menos 200 unidades
  # indicator (arrow syntax): an open site ships at least 200 units
  despacho_min{i en Sitios}: abrir[i] = 1 -> suma{j en Clientes} x[i,j] >= 200;
  # a lo sumo tres centros / at most three sites
  pocos_centros: atmost(3, abrir[Bogota], abrir[Medellin], abrir[Cali], abrir[Barranquilla]);
`
  },
  {
    id: 'secuencia',
    titulo: 'Secuenciación con disyunciones',
    resumen: 'Una máquina sin traslapes: «inicio[i] + p[i] <= inicio[j] o …».',
    texto: `# Programación de tareas en una máquina con disyunciones (sin traslapes).
# Single-machine scheduling with disjunctions (no overlap).
# Original de IAQSB (datos ilustrativos / illustrative data).

conjunto Tareas = 1..5;
parametro p{Tareas}   = [6, 4, 8, 3, 5];        # duración / processing time
parametro d{Tareas}   = [10, 8, 25, 6, 20];     # fecha de entrega / due date
parametro peso{Tareas} = [2, 1, 1, 3, 2];       # importancia / weight

variable inicio{Tareas} >= 0 <= 40;             # instante de inicio / start time
variable atraso{Tareas} >= 0 <= 40;             # tardanza / tardiness

minimizar atraso_ponderado: suma{j en Tareas} peso[j] * atraso[j];

sujeto a
  # atraso = máx(0, fin − entrega) / tardiness = max(0, completion − due date)
  tarde{j en Tareas}: atraso[j] >= inicio[j] + p[j] - d[j];
  # disyunción: i va antes que j, o j antes que i / disjunction: i before j, or j before i
  orden{i en Tareas, j en Tareas: i < j}:
    inicio[i] + p[i] <= inicio[j] o inicio[j] + p[j] <= inicio[i];
`
  },
  {
    id: 'descuentos',
    titulo: 'Descuentos por cantidad',
    resumen: 'Costo lineal por tramos con pwl() y un cargo fijo.',
    texto: `# Compras con descuentos por cantidad: costo lineal por tramos (pwl).
# Purchasing with quantity discounts: piecewise-linear cost (pwl).
# Original de IAQSB (datos ilustrativos / illustrative data).

# Proveedor A: 10 por unidad hasta 200, luego 8, y desde 500 a 6.
# Supplier A: 10 per unit up to 200, then 8, and from 500 on, 6.
# Proveedor B: tarifa plana de 9 con un cargo de 300 si se le compra algo.
# Supplier B: flat 9 per unit plus a 300 charge if anything is bought.

var qa >= 0 <= 800;          # unidades al proveedor A / units from A
var qb >= 0 <= 800;          # unidades al proveedor B / units from B
var usa_b binary;            # 1 si se le compra a B / 1 if B is used

minimizar costo:
  pwl(qa; (0, 0), (200, 2000), (500, 4400), (800, 6200))
  + 9 * qb + 300 * usa_b;

sujeto a
  demanda: qa + qb >= 650;
  # sin cargo fijo, B no puede despachar / B ships nothing without the fixed charge
  activa_b: usa_b = 0 -> qb <= 0;
  # política: ningún proveedor supera el 80 % / policy: no supplier above 80 %
  tope_a: qa <= 0.8 * (qa + qb);
`
  },
  {
    id: 'rapido',
    titulo: 'Modo rápido (dos productos)',
    resumen: 'Sin declarar variables: max 3x + 5y; … (con método gráfico).',
    texto: `# Modo rápido: sin declarar variables (x, y se toman como continuas ≥ 0).
# Quick mode: no declarations needed (x, y become continuous ≥ 0).
# Original de IAQSB: dos productos y tres plantas / two products, three plants.

max 3x + 5y;
s.t.
planta1: x <= 4;
planta2: 2y <= 12;
planta3: 3x + 2y <= 18;
`
  }
];

/** Plantilla de «Nuevo»: un modelo mínimo que ya se puede resolver. */
export const PLANTILLA_ALGEBRA = `# Modelo nuevo: escríbelo aquí y resuélvelo con Ctrl+Intro.
# New model: write it here and solve it with Ctrl+Enter.

conjunto I = {A, B, C};
parametro ganancia{I} = [5, 4, 3];
parametro uso{I} = [2, 3, 1];

variable x{I} >= 0;

maximizar z: suma{i en I} ganancia[i] * x[i];

sujeto a
  recurso: suma{i en I} uso[i] * x[i] <= 100;
`;
