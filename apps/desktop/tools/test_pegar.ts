// Pruebas de la lógica de pegado desde Excel (src/lib/pegar.ts), sin dependencias.
//
// Uso (Node 22.18 o posterior, que entiende TypeScript):
//   node tools/test_pegar.ts

import * as P from '../src/lib/pegar.ts';
import type { LpModel } from '../src/lib/types.ts';
import type { Field } from '../src/lib/gtypes.ts';

let fails = 0;
let passes = 0;
function ok(cond: unknown, what: string) {
  if (cond) passes++;
  else {
    fails++;
    console.error('  FALLA: ' + what);
  }
}
function eq(a: unknown, b: unknown, what: string) {
  const sa = JSON.stringify(a);
  const sb = JSON.stringify(b);
  ok(sa === sb, `${what}: ${sa} ≠ ${sb}`);
}

// ---------------------------------------------------------------- texto
eq(P.leerTabla('1\t2\t3\r\n4\t5\t6\r\n'), [['1', '2', '3'], ['4', '5', '6']], 'TSV de Excel con salto final');
eq(P.leerTabla('a;b\n"x;y";"dos\nlíneas"'), [['a', 'b'], ['x;y', 'dos\nlíneas']], 'punto y coma y comillas');
eq(P.leerTabla('"di ""hola"""\t2'), [['di "hola"', '2']], 'comillas dobles escapadas');
eq(P.leerTabla('3 5 4'), [['3', '5', '4']], 'números separados por espacios');
eq(P.leerTabla('Mano de obra'), [['Mano de obra']], 'texto con espacios queda en una celda');
eq(P.leerTabla('1\t2\t\t\n3\t\t\t\n\n'), [['1', '2'], ['3', '']], 'columnas y filas vacías al final');
eq(P.leerTabla('\tX1\tX2\nMax\t3\t5'), [['', 'X1', 'X2'], ['Max', '3', '5']], 'esquina vacía');

// ---------------------------------------------------------------- números
eq(P.numero('3,5', true), 3.5, 'coma decimal');
eq(P.numero('3.5', false), 3.5, 'punto decimal');
eq(P.numero('3,5', false), 3.5, 'coma decimal aunque se prefiera punto');
eq(P.numero('1.234,5', false), 1234.5, 'miles con punto');
eq(P.numero('1,234.5', true), 1234.5, 'miles con coma');
eq(P.numero('1.234', true), 1234, 'ambiguo con coma decimal = miles');
eq(P.numero('1.234', false), 1.234, 'ambiguo con punto decimal = decimal');
eq(P.numero('1,234', false), 1234, 'ambiguo 1,234 con punto decimal = miles');
eq(P.numero('1.234.567', false), 1234567, 'varios puntos = miles');
eq(P.numero('15%', true), 0.15, 'porcentaje');
eq(P.numero('12,5 %', true), 0.125, 'porcentaje con coma');
eq(P.numero('−2', true), -2, 'signo menos tipográfico');
eq(P.numero('(4)', true), -4, 'negativo contable');
eq(P.numero('$ 1.200', true), 1200, 'moneda');
eq(P.numero('1 234,5', true), 1234.5, 'espacio de miles');
eq(P.numero('1,5E-3', true), 0.0015, 'notación científica con coma');
eq(P.numero('abc', true), null, 'texto no es número');
eq(P.numero('1..2', true), null, 'dos puntos seguidos');
eq(P.numero('-', true), null, 'solo el signo');
eq(P.detectarComa([['1,5', '2'], ['3', '4']], false), true, 'detecta coma');
eq(P.detectarComa([['1.5', '2']], true), false, 'detecta punto');
eq(P.detectarComa([['1.234', '2']], true), true, 'ambiguo: usa la preferencia');
eq(P.cota('M', true), 'M', 'cota M');
eq(P.cota('-M', true), '-M', 'cota -M');
eq(P.cota('∞', true), 'M', 'cota infinito');
eq(P.cota('2,5', true), 2.5, 'cota numérica');
eq(P.sentido('≤'), '<=', 'sentido ≤');
eq(P.sentido('>='), '>=', 'sentido >=');
eq(P.tipoVariable('Entera'), 'integer', 'tipo entera');
eq(P.tipoVariable('Binária'), 'binary', 'tipo binária');
eq(P.tipoVariable('Semicontinua'), 'semicontinuous', 'tipo semicontinua');

// ---------------------------------------------------------------- LP-ILP
function lp(nv = 2, nc = 2): LpModel {
  return {
    name: 'P',
    objective: 'max',
    variables: Array.from({ length: nv }, (_, j) => ({ name: `X${j + 1}`, cost: 0, lower: 0, upper: 'M' as const, type: 'continuous' as const })),
    constraints: Array.from({ length: nc }, (_, i) => ({ name: `C${i + 1}`, coefs: Array(nv).fill(0), direction: '<=' as const, rhs: 0 }))
  };
}

{
  // bloque de coeficientes en la celda del objetivo de X1
  const t = P.leerTabla('3\t5\n1\t0\n0\t2');
  const a = P.analizarLP(lp(), 1, 1, t, false);
  eq(a.falta, { filas: 0, columnas: 0, ultimaLD: false }, 'cabe justo');
  const r = P.pegarLP(lp(), a, { crecer: false });
  eq(r.modelo.variables.map((v) => v.cost), [3, 5], 'costos');
  eq(r.modelo.constraints.map((c) => c.coefs), [[1, 0], [0, 2]], 'coeficientes');
  eq(r.escritas.length, 6, 'celdas escritas');
}
{
  // la tabla completa de Wyndor, con encabezado, nombres, sentido y lado derecho
  const txt = '\tPuertas\tVentanas\t\tLD\nMax\t3\t5\t\t\nPlanta1\t1\t0\t<=\t4\nPlanta2\t0\t2\t<=\t12\nPlanta3\t3\t2\t<=\t18\n';
  const t = P.leerTabla(txt);
  const a = P.analizarLP(lp(), 1, 1, t, false);
  ok(a.encabezado && a.nombres, 'detecta encabezado y nombres');
  eq(a.colSentido, 2, 'columna del sentido');
  eq(a.falta, { filas: 1, columnas: 0, ultimaLD: false }, 'falta una restricción');
  const r = P.pegarLP(lp(), a, { crecer: true });
  const m = r.modelo;
  eq(m.variables.map((v) => v.name), ['Puertas', 'Ventanas'], 'renombra variables');
  eq(m.constraints.map((c) => c.name), ['Planta1', 'Planta2', 'Planta3'], 'renombra y añade restricciones');
  eq(m.constraints.map((c) => [c.coefs, c.direction, c.rhs]), [[[1, 0], '<=', 4], [[0, 2], '<=', 12], [[3, 2], '<=', 18]], 'restricciones de Wyndor');
  eq(m.variables.map((v) => v.cost), [3, 5], 'costos de Wyndor');
  eq(r.nuevasFilas, 1, 'una restricción nueva');
  // anclada en el nombre de la primera variable (fila 0): igual
  const b = P.pegarLP(lp(), P.analizarLP(lp(), 0, 0, t, false), { crecer: true });
  eq(b.modelo.constraints.map((c) => c.rhs), [4, 12, 18], 'anclada en la esquina');
}
{
  // sin crecer: lo que no cabe queda fuera
  const t = P.leerTabla('1\t2\t3\n4\t5\t6\n7\t8\t9');
  const a = P.analizarLP(lp(), 2, 1, t, true);
  eq(a.falta.columnas, 1, 'falta una variable');
  ok(a.falta.ultimaLD, 'ofrece la última columna como lado derecho');
  const r = P.pegarLP(lp(), a, { crecer: false });
  ok(r.fuera > 0, 'cuenta lo que no cupo');
  const r2 = P.pegarLP(lp(), a, { crecer: true, ultimaLD: true });
  eq(r2.modelo.variables.length, 2, 'lado derecho: no añade variables');
  eq(r2.modelo.constraints.map((c) => c.rhs), [3, 6, 9], 'lado derecho pegado');
  eq(r2.modelo.constraints.length, 3, 'añade la restricción que falta');
  const r3 = P.pegarLP(lp(), a, { crecer: true });
  eq(r3.modelo.variables.map((v) => v.name), ['X1', 'X2', 'X3'], 'añade X3');
  eq(r3.modelo.constraints[2].coefs, [7, 8, 9], 'fila nueva completa');
}
{
  // n+1 columnas con números: la última va al lado derecho
  const t = P.leerTabla('1\t2\t10\n3\t4\t20');
  const a = P.analizarLP(lp(), 2, 1, t, true);
  const r = P.pegarLP(lp(), a, { crecer: false });
  // positional: la columna que cae en el sentido no tiene sentidos → se ofrece como LD
  eq(a.falta.columnas, 1, 'pasa en una columna');
  const r2 = P.pegarLP(lp(), a, { crecer: true, ultimaLD: true });
  eq(r2.modelo.constraints.map((c) => c.rhs), [10, 20], 'LD');
  ok(r.fuera === 2, 'sin crecer, lo que sobra queda fuera');
}
{
  // cotas y tipo pegados con sus rótulos
  const t = P.leerTabla('Cota inferior\t1\t-M\nCota superior\tM\t10\nTipo\tEntera\tLibre');
  const a = P.analizarLP(lp(), 4, 1, t, true);
  const m = P.pegarLP(lp(), a, { crecer: false }).modelo;
  eq(m.variables.map((v) => [v.lower, v.upper, v.type]), [[1, 'M', 'integer'], ['-M', 'M', 'unrestricted']], 'cotas y tipos');
}
{
  // coma decimal y celda inválida
  const t = P.leerTabla('2,5\tabc');
  const a = P.analizarLP(lp(), 1, 1, t, true);
  const r = P.pegarLP(lp(), a, { crecer: false });
  eq(r.modelo.variables[0].cost, 2.5, 'coma decimal en el objetivo');
  eq(r.invalidas, 1, 'una inválida');
  eq(r.modelo.variables[1].cost, 0, 'la inválida queda como estaba');
}
{
  // columna de nombres pegada en la columna de nombres
  const t = P.leerTabla('Mano de obra\nMadera');
  const r = P.pegarLP(lp(), P.analizarLP(lp(), 2, 0, t, true), { crecer: false });
  eq(r.modelo.constraints.map((c) => c.name), ['Mano de obra', 'Madera'], 'nombres de restricciones');
}
{
  // minimizar desde el rótulo del objetivo
  const t = P.leerTabla('Min\t1\t2');
  const r = P.pegarLP(lp(), P.analizarLP(lp(), 1, 0, t, true), { crecer: false });
  eq(r.modelo.objective, 'min', 'objetivo min');
}

// ---------------------------------------------------------------- matriz (transporte)
const num: Field = { key: 'v', label: 'Costo', kind: { type: 'number' } };
const W: P.MatrixW = {
  type: 'matrix',
  path: 'costs',
  rows: { path: 'sources', label_key: 'name', label: 'Origen', extra: [{ key: 'supply', label: 'Oferta', kind: { type: 'number' } }], add_label: 'Añadir origen' },
  cols: { path: 'dests', label_key: 'name', label: 'Destino', extra: [{ key: 'demand', label: 'Demanda', kind: { type: 'number' } }], add_label: 'Añadir destino' },
  cell: num
};
function tr() {
  return {
    sources: [{ name: 'O1', supply: 0 }, { name: 'O2', supply: 0 }],
    dests: [{ name: 'D1', demand: 0 }, { name: 'D2', demand: 0 }],
    costs: [[0, 0], [0, 0]]
  };
}
{
  // tabla completa con encabezado, nombres, oferta y demanda; una columna más
  const t = P.leerTabla('\tBogotá\tCali\tMedellín\tOferta\nPlanta A\t4\t6\t9\t100\nPlanta B\t5\t3\t7\t150\nDemanda\t80\t70\t100\t');
  const a = P.analizarMatriz(W, tr(), 0, 0, t, true, true, true);
  ok(a.encabezado && a.nombres, 'matriz: encabezado y nombres');
  eq(a.falta, { filas: 0, columnas: 1, ultimaLD: false }, 'matriz: falta un destino');
  const r = P.pegarMatriz(tr(), W, [], a, { crecer: true }, { filas: 'Origen', cols: 'Destino' });
  const m = r.modelo;
  eq(m.dests.map((d: any) => [d.name, d.demand]), [['Bogotá', 80], ['Cali', 70], ['Medellín', 100]], 'destinos y demanda');
  eq(m.sources.map((s: any) => [s.name, s.supply]), [['Planta A', 100], ['Planta B', 150]], 'orígenes y oferta');
  eq(m.costs, [[4, 6, 9], [5, 3, 7]], 'costos');
}
{
  // sin encabezado: bloque más grande, las últimas columnas siguen siendo la oferta
  const t = P.leerTabla('1\t2\t3\t10\n4\t5\t6\t20\n7\t8\t9\t30');
  const a = P.analizarMatriz(W, tr(), 0, 0, t, false, true, true);
  eq(a.falta, { filas: 1, columnas: 1, ultimaLD: false }, 'falta un origen y un destino');
  const r = P.pegarMatriz(tr(), W, [], a, { crecer: true }, { filas: 'Origen', cols: 'Destino' });
  eq(r.modelo.costs, [[1, 2, 3], [4, 5, 6], [7, 8, 9]], 'costos 3×3');
  eq(r.modelo.sources.map((s: any) => s.supply), [10, 20, 30], 'oferta');
  eq(r.modelo.sources.map((s: any) => s.name), ['O1', 'O2', 'O3'], 'nombre nuevo O3');
  const r2 = P.pegarMatriz(tr(), W, [], a, { crecer: false }, { filas: 'Origen', cols: 'Destino' });
  eq(r2.modelo.costs, [[1, 2], [4, 5]], 'sin crecer: solo lo que cabe');
}
{
  // matriz cuadrada (distancias): crecer añade filas y columnas
  const Q: P.MatrixW = { type: 'matrix', path: 'd', rows: { path: 'n', label_key: 'name', label: 'Nodo', extra: [], add_label: 'Añadir nodo' }, cols: { path: 'n', label_key: 'name', label: 'Nodo', extra: [] }, cell: { key: 'd', label: 'd', kind: { type: 'number' }, optional: true } };
  const m0 = { n: [{ name: 'N1' }, { name: 'N2' }], d: [[null, 1], [1, null]] };
  const t = P.leerTabla('0\t1\t2\n1\t0\t3\n2\t3\t0');
  const a = P.analizarMatriz(Q, m0, 0, 0, t, false, true, false);
  eq(a.falta, { filas: 1, columnas: 1, ultimaLD: false }, 'cuadrada: un nodo más');
  const r = P.pegarMatriz(m0, Q, [], a, { crecer: true }, { filas: 'Nodo', cols: 'Nodo' });
  eq(r.modelo.d, [[null, 1, 2], [1, null, 3], [2, 3, null]], 'cuadrada: diagonal intacta');
  eq(r.modelo.n.map((x: any) => x.name), ['N1', 'N2', 'N3'], 'cuadrada: N3');
}

// ---------------------------------------------------------------- registros
{
  const cols: Field[] = [
    { key: 'name', label: 'Actividad', kind: { type: 'text' } },
    { key: 'time', label: 'Duración', kind: { type: 'number' } },
    { key: 'pred', label: 'Predecesoras', kind: { type: 'list' } }
  ];
  const w: P.RecordsW = { type: 'records', path: 'acts', columns: cols, add_label: 'Añadir actividad' };
  const m0 = { acts: [{ name: 'A', time: 1, pred: [] }] };
  const t = P.leerTabla('Actividad\tPredecesoras\tDuración\nA\t\t3\nB\tA\t4,5\nC\tA, B\t2');
  const a = P.analizarRegistros(m0, w, cols, 0, 0, t, true, true);
  ok(a.encabezado, 'registros: encabezado por rótulos');
  eq(a.falta.filas, 2, 'registros: faltan dos');
  const r = P.pegarRegistros(m0, w, cols, a, { crecer: true }, 'Elemento');
  eq(r.modelo.map((x: any) => [x.name, x.time, x.pred]), [['A', 3, []], ['B', 4.5, ['A']], ['C', 2, ['A', 'B']]], 'registros por rótulo');
}

console.log(`${passes} bien, ${fails} fallas`);
process.exit(fails ? 1 : 0);
