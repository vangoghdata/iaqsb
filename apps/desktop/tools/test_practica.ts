// Pruebas de la lógica del modo práctica (src/lib/practica.ts), sin dependencias.
//
// Uso (Node 22.18 o posterior, que entiende TypeScript):
//   cargo build -p iaqsb
//   node tools/test_practica.ts [ruta/a/iaqsb]
//
// La tabla inicial y los óptimos de referencia salen del núcleo (la CLI iaqsb),
// igual que en la aplicación; el pivoteo, las comprobaciones y el branch and
// bound los hace practica.ts.

import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import * as P from '../src/lib/practica.ts';
import type { LpModel, LpReport } from '../src/lib/types.ts';

const here = dirname(fileURLToPath(import.meta.url));
const CLI = process.argv[2] ?? resolve(here, '../../../target/debug/iaqsb');
if (!existsSync(CLI)) {
  console.error(`No está la CLI en ${CLI}: ejecuta «cargo build -p iaqsb».`);
  process.exit(2);
}

function solve(model: LpModel, options: Record<string, unknown>): LpReport {
  const out = execFileSync(CLI, ['--json', 'llamar', 'solve'], { input: JSON.stringify({ module: 'lp', model, options, lang: 'es' }) }).toString();
  const j = JSON.parse(out);
  if (!j.ok) throw new Error(j.error?.message ?? 'error del núcleo');
  return j.result as LpReport;
}

let fails = 0;
let passes = 0;
function ok(cond: unknown, what: string) {
  if (cond) passes++;
  else {
    fails++;
    console.error('  FALLA: ' + what);
  }
}
function eq<T>(a: T, b: T, what: string) {
  ok(a === b, `${what}: ${String(a)} ≠ ${String(b)}`);
}
function section(name: string) {
  console.log('· ' + name);
}

const v = (name: string, cost: number, extra: Partial<LpModel['variables'][0]> = {}) => ({ name, cost, lower: 0 as const, upper: 'M' as const, type: 'continuous' as const, ...extra });
const c = (name: string, coefs: number[], direction: '<=' | '>=' | '=', rhs: number) => ({ name, coefs, direction, rhs });

const wyndor: LpModel = {
  name: 'Wyndor',
  objective: 'max',
  variables: [v('X1', 3), v('X2', 5)],
  constraints: [c('C1', [1, 0], '<=', 4), c('C2', [0, 2], '<=', 12), c('C3', [3, 2], '<=', 18)]
};
// Terapia por radiación (Hillier): ≤, = y ≥ con gran M y coeficientes decimales
const radiacion: LpModel = {
  name: 'Radiación',
  objective: 'min',
  variables: [v('X1', 0.4), v('X2', 0.5)],
  constraints: [c('C1', [0.3, 0.1], '<=', 2.7), c('C2', [0.5, 0.5], '=', 6), c('C3', [0.6, 0.4], '>=', 6)]
};
const infactible: LpModel = {
  name: 'Infactible',
  objective: 'max',
  variables: [v('X1', 1), v('X2', 1)],
  constraints: [c('C1', [1, 1], '<=', 2), c('C2', [1, 1], '>=', 5)]
};
const noAcotado: LpModel = {
  name: 'No acotado',
  objective: 'max',
  variables: [v('X1', 1), v('X2', 1)],
  constraints: [c('C1', [1, -1], '<=', 1), c('C2', [-1, 1], '<=', 2)]
};
// cota inferior x1 ≥ 2 (desplazamiento) y minimización con ≥
const dieta: LpModel = {
  name: 'Dieta',
  objective: 'min',
  variables: [v('X1', 2, { lower: 2 }), v('X2', 3)],
  constraints: [c('C1', [1, 1], '>=', 6), c('C2', [1, 3], '>=', 8)]
};
// degenerado: empate en el cociente mínimo
const degenerado: LpModel = {
  name: 'Degenerado',
  objective: 'max',
  variables: [v('X1', 2), v('X2', 1)],
  constraints: [c('C1', [1, 1], '<=', 4), c('C2', [1, 0], '<=', 4), c('C3', [0, 1], '<=', 3)]
};

/** Tabla inicial exacta (iteración 0 del núcleo) y el informe del núcleo. */
function initial(model: LpModel): { tb: P.PTableau; rep: LpReport } {
  const rep = solve(model, { trace: true, relax_integers: true, diagnose: false });
  if (!rep.simplex_trace?.length) throw new Error('el núcleo no devolvió las tablas');
  return { tb: P.fromSnapshot(rep.simplex_trace[0], model.objective), rep };
}

type Pick = (tb: P.PTableau, cand: number[]) => number;
const dantzigPick: Pick = (tb) => P.dantzig(tb)[0];
const firstPick: Pick = (_tb, cand) => cand[0];
const lastPick: Pick = (_tb, cand) => cand[cand.length - 1];

/** Recorre el símplex con una regla de entrada; la de salida es el cociente mínimo. */
function run(tb: P.PTableau, pick: Pick, maxIter = 50): { end: P.Conclusion; tb: P.PTableau; steps: number } {
  for (let k = 0; k < maxIter; k++) {
    if (P.provablyInfeasible(tb)) return { end: 'infeasible', tb, steps: k };
    const cand = P.improving(tb);
    if (!cand.length) return { end: P.positiveArtificials(tb).length ? 'infeasible' : 'optimal', tb, steps: k };
    const e = pick(tb, cand);
    const v = P.checkEntering(tb, e);
    ok(v.ok, `entrada válida ${tb.columns[e]}`);
    const rows = P.minRatioRows(tb, e);
    if (!rows.length) return { end: P.positiveArtificials(tb).length ? 'infeasible' : 'unbounded', tb, steps: k };
    ok(P.checkLeaving(tb, e, rows[0]).ok, 'salida válida');
    tb = P.pivot(tb, rows[0], e);
    // invariantes: identidad en las básicas y lado derecho no negativo
    tb.basis.forEach((j, r) => tb.rows.forEach((row, i) => ok(row[j].eq(i === r ? P.Frac.ONE : P.Frac.ZERO), 'columna básica unitaria')));
    ok(tb.rhs.every((b) => b.sign() >= 0), 'lado derecho no negativo');
  }
  throw new Error('demasiadas iteraciones');
}

/** Todos los caminos posibles (toda entrada válida × toda fila empatada) terminan en el mismo óptimo. */
function allPaths(tb: P.PTableau, depth = 0, acc: number[] = []): number[] {
  if (depth > 12) throw new Error('camino demasiado largo');
  const cand = P.improving(tb);
  if (!cand.length || P.provablyInfeasible(tb)) {
    acc.push(P.objective(tb).c.toNumber());
    return acc;
  }
  for (const e of cand) {
    const rows = P.minRatioRows(tb, e);
    if (!rows.length) continue;
    for (const r of rows) allPaths(P.pivot(tb, r, e), depth + 1, acc);
  }
  return acc;
}

// ------------------------------------------------------------ fracciones

section('Fracciones y valores con M');
{
  const F = P.Frac;
  eq(new F(6n, -4n).toString(), '−3/2', 'normaliza signo y factores');
  eq(P.fracOf(0.25).toString(), '1/4', '0.25 → 1/4');
  eq(P.fracOf(1.333333333).toString(), '4/3', '1.333333333 → 4/3');
  eq(P.fracOf(0.1).toString(), '1/10', '0.1 → 1/10');
  eq(P.fracOf(2.7).toString(), '27/10', '2.7 → 27/10');
  eq(P.parseFrac('1/2')?.toString(), '1/2', 'lee 1/2');
  eq(P.parseFrac(' −3/4 ')?.toString(), '−3/4', 'lee −3/4');
  eq(P.parseFrac('0,5')?.toString(), '1/2', 'lee 0,5');
  eq(P.parseFrac('1.5/3')?.toString(), '1/2', 'lee 1.5/3');
  eq(P.parseFrac('abc'), null, 'rechaza texto');
  eq(P.parseFrac('1/0'), null, 'rechaza división entre cero');
  ok(P.closeEnough(P.parseFrac('0.333')!, new F(1n, 3n)), '0.333 ≈ 1/3');
  ok(P.closeEnough(P.parseFrac('0.67')!, new F(2n, 3n)), '0.67 ≈ 2/3');
  ok(!P.closeEnough(P.parseFrac('0.6')!, new F(2n, 3n)), '0.6 ≉ 2/3');
  const a = P.mv(F.int(2), F.int(-3));
  eq(P.fmtMV(a, true), '2M − 3', 'formato 2M − 3');
  eq(P.fmtMV(P.mv(F.int(-1), F.ZERO), true), '−M', 'formato −M');
  eq(P.fmtMV(P.mv(F.ZERO, new F(1n, 2n)), false), '0.5', 'formato decimal');
  ok(P.mvCmp(P.mv(F.int(1), F.int(-1000)), P.mv(F.ZERO, F.int(1000000))) > 0, 'M − 1000 > 1000000');
}

// ------------------------------------------------------------ Wyndor

section('Wyndor (max 3X1 + 5X2 → z = 36)');
{
  const { tb, rep } = initial(wyndor);
  eq(rep.objective_value, 36, 'óptimo del núcleo');
  eq(P.objective(tb).c.toNumber(), 0, 'z inicial = 0');
  // decisiones paso a paso
  const x1 = tb.columns.indexOf('X1');
  const x2 = tb.columns.indexOf('X2');
  const s1 = tb.columns.findIndex((n) => n.endsWith('C1'));
  eq(P.checkEntering(tb, x2).level, 'ok', 'X2 es la de Dantzig');
  eq(P.checkEntering(tb, x1).level, 'warn', 'X1 es válida pero no Dantzig');
  ok(P.checkEntering(tb, x1).msgs.some((m) => m.args?.includes('X2')), 'la explicación menciona X2');
  eq(P.checkEntering(tb, s1).ok, false, 'una holgura básica no entra');
  eq(P.checkDeclaration(tb, 'optimal').ok, false, 'la tabla inicial no es óptima');
  // fila que sale para X2: C2 (12/2 = 6); C3 da 18/2 = 9; C1 tiene 0
  eq(P.minRatioRows(tb, x2).join(), '1', 'cociente mínimo en la fila de C2');
  eq(P.checkLeaving(tb, x2, 2).ok, false, 'C3 no es el mínimo');
  eq(P.checkLeaving(tb, x2, 0).ok, false, 'C1 tiene 0 en la columna');
  eq(P.checkLeaving(tb, x2, 1).ok, true, 'C2 es la correcta');
  // fila pivote escrita por el estudiante (columnas y lado derecho)
  const exact = P.newPivotRow(tb, 1, x2).map((f) => f.toString());
  eq(exact.join(' '), '0 1 0 1/2 0 6', 'nueva fila pivote exacta');
  eq(P.checkPivotRow(tb, 1, x2, ['0', '1', '0', '0,5', '0', '6']).ok, true, 'fila pivote con decimales');
  const bad = P.checkPivotRow(tb, 1, x2, ['0', '1', '0', '2', '0', '6']);
  eq(bad.ok, false, 'fila pivote con un error');
  eq(bad.cells.join(), 'true,true,true,false,true,true', 'marca la casilla mal');
  // tres caminos distintos
  for (const [name, pick] of [
    ['Dantzig', dantzigPick],
    ['primera que mejora', firstPick],
    ['última que mejora', lastPick]
  ] as [string, Pick][]) {
    const r = run(tb, pick);
    eq(r.end, 'optimal', `${name}: óptimo`);
    eq(P.objective(r.tb).c.toNumber(), 36, `${name}: z = 36`);
    ok(P.checkDeclaration(r.tb, 'optimal').ok, `${name}: se acepta «óptimo»`);
    const vals = P.variableValues(wyndor, r.tb).map((x) => x.toString());
    eq(vals.join(), '2,6', `${name}: X1 = 2, X2 = 6`);
  }
  eq(run(tb, dantzigPick).steps, 2, 'Dantzig: 2 iteraciones');
  eq(run(tb, firstPick).steps, 3, 'entrando X1 primero: 3 iteraciones');
  const ends = allPaths(tb);
  ok(ends.length >= 2 && ends.every((z) => z === 36), `todos los caminos llegan a 36 (${ends.length} caminos)`);
}

section('Radiación (min con ≤, = y ≥: gran M → z = 5.25)');
{
  const { tb, rep } = initial(radiacion);
  eq(rep.objective_value, 5.25, 'óptimo del núcleo');
  ok(P.objective(tb).m.sign() > 0, 'z inicial con M positiva (minimizar)');
  ok(tb.columns.some((_, j) => P.isArtificial(tb, j)), 'hay artificiales');
  for (const pick of [dantzigPick, firstPick, lastPick]) {
    const r = run(tb, pick);
    eq(r.end, 'optimal', 'óptimo');
    ok(P.objective(r.tb).m.isZero(), 'sin M en el óptimo');
    eq(P.objective(r.tb).c.toString(), '21/4', 'z = 21/4 exacto');
    eq(P.variableValues(radiacion, r.tb).map((x) => x.toString()).join(), '15/2,9/2', 'X1 = 7.5, X2 = 4.5');
  }
  ok(allPaths(tb).every((z) => z === 5.25), 'todos los caminos llegan a 5.25');
}

section('Dieta (min, ≥, cota inferior X1 ≥ 2)');
{
  const { tb, rep } = initial(dieta);
  const r = run(tb, firstPick);
  eq(r.end, 'optimal', 'óptimo');
  ok(P.sameObjective(P.objective(r.tb).c.toNumber(), rep.objective_value), `z = ${rep.objective_value} como el núcleo`);
  const vals = P.variableValues(dieta, r.tb).map((x) => x.toNumber());
  ok(vals[0] >= 2, 'X1 respeta la cota inferior');
  ok(P.sameObjective(2 * vals[0] + 3 * vals[1], rep.objective_value), 'los valores reproducen z');
}

section('Infactible');
{
  const { tb, rep } = initial(infactible);
  eq(rep.status, 'infeasible', 'el núcleo dice no factible');
  for (const pick of [dantzigPick, firstPick, lastPick]) {
    const r = run(tb, pick);
    eq(r.end, 'infeasible', 'termina en no factible');
    eq(P.checkDeclaration(r.tb, 'optimal').ok, false, '«óptimo» se rechaza');
    eq(P.checkDeclaration(r.tb, 'infeasible').ok, true, '«no factible» se acepta');
    eq(P.expectedConclusion(r.tb), 'infeasible', 'conclusión esperada');
  }
  eq(P.checkDeclaration(tb, 'infeasible').ok, false, 'en la tabla inicial aún no se puede concluir');
}

section('No acotado');
{
  const { tb, rep } = initial(noAcotado);
  eq(rep.status, 'unbounded', 'el núcleo dice no acotado');
  let t = tb;
  let guard = 0;
  while (guard++ < 10) {
    const e = P.dantzig(t)[0];
    if (e === undefined) break;
    if (P.noPositive(t, e)) break;
    t = P.pivot(t, P.minRatioRows(t, e)[0], e);
  }
  eq(P.checkDeclaration(t, 'unbounded').ok, true, '«no acotado» se acepta');
  eq(P.checkDeclaration(t, 'optimal').ok, false, '«óptimo» se rechaza');
  eq(P.expectedConclusion(t), 'unbounded', 'conclusión esperada');
  const e = P.dantzig(t)[0];
  eq(P.checkLeaving(t, e, 0).ok, false, 'ninguna fila puede salir');
  eq(P.checkDeclaration(tb, 'unbounded').ok, false, 'en la tabla inicial todavía hay cociente');
}

section('Degenerado (empate en el cociente)');
{
  const { tb, rep } = initial(degenerado);
  const x1 = tb.columns.indexOf('X1');
  const rows = P.minRatioRows(tb, x1);
  eq(rows.length, 2, 'dos filas empatadas');
  for (const r of rows) {
    const v = P.checkLeaving(tb, x1, r);
    ok(v.ok && v.level === 'warn', 'cualquier fila empatada es válida, con aviso');
  }
  ok(allPaths(tb).every((z) => P.sameObjective(z, rep.objective_value)), 'todos los caminos llegan al óptimo');
}

// ------------------------------------------------------------ branch and bound

section('Branch and bound (decisiones)');
{
  const m: LpModel = {
    name: 'Entero',
    objective: 'max',
    variables: [v('X1', 5, { type: 'integer' }), v('X2', 8, { type: 'integer' }), v('X3', 1)],
    constraints: [c('C1', [1, 1, 0], '<=', 6), c('C2', [5, 9, 0], '<=', 45), c('C3', [0, 0, 1], '<=', 1)]
  };
  const node = (status: P.NodeStatus, z: number | null, values: number[]): P.BNode => ({ id: 1, parent: null, depth: 0, branch: null, bounds: [], status, z, values });
  const tol = 0.01;
  eq(P.correctDecision(m, node('infeasible', null, []), null, tol).decision, 'prune_infeasible', 'no factible → podar');
  eq(P.correctDecision(m, node('optimal', 41.25, [2.25, 3.75, 1]), null, tol).decision, 'branch', 'fraccionaria → ramificar');
  eq(P.correctDecision(m, node('optimal', 40, [0, 5, 0.5]), null, tol).decision, 'incumbent', 'X3 continua fraccionaria no importa');
  eq(P.correctDecision(m, node('optimal', 40, [0, 5, 0]), 41, tol).decision, 'prune_bound', 'entera pero peor → podar por cota');
  eq(P.correctDecision(m, node('optimal', 41, [3, 3, 1]), 41, tol).decision, 'prune_bound', 'igual a la incumbente → podar por cota');
  eq(P.correctDecision(m, node('optimal', 41.2, [2.5, 3, 1]), 41, tol).decision, 'branch', 'mejor y fraccionaria → ramificar');
  eq(P.checkDecision(m, node('optimal', 41.25, [2.25, 3.75, 1]), null, tol, 'prune_bound').ok, false, 'sin incumbente no se poda por cota');
  const n = node('optimal', 41.25, [2.25, 3.75, 0.5]);
  eq(P.checkBranchVar(m, n, 0, tol).ok, true, 'X1 fraccionaria sirve');
  eq(P.checkBranchVar(m, n, 1, tol).ok, true, 'X2 fraccionaria sirve');
  eq(P.checkBranchVar(m, n, 2, tol).ok, false, 'X3 continua no sirve');
  eq(P.checkBranchVar(m, node('optimal', 40, [2, 3.5, 0]), 0, tol).ok, false, 'X1 entera no sirve');
  const { down, up, fl, ce } = P.childBounds([[0, Infinity], [0, Infinity], [0, Infinity]], 1, 3.75);
  eq(`${fl} ${ce} ${down[1].join()} ${up[1].join()}`, '3 4 0,3 4,Infinity', 'hijos X2 ≤ 3 y X2 ≥ 4');
  const mb = P.modelWithBounds(m, up);
  eq(`${mb.variables[1].lower} ${mb.variables[1].upper}`, '4 M', 'modelo con cotas');
  const pos = P.layoutTree([
    { ...node('optimal', 1, []), id: 1 },
    { ...node('optimal', 1, []), id: 2, parent: 1, depth: 1 },
    { ...node('optimal', 1, []), id: 3, parent: 1, depth: 1 },
    { ...node('optimal', 1, []), id: 4, parent: 2, depth: 2 },
    { ...node('optimal', 1, []), id: 5, parent: 2, depth: 2 }
  ]);
  eq(`${pos.get(1)!.x} ${pos.get(2)!.x} ${pos.get(3)!.x}`, '1.25 0.5 2', 'disposición del árbol');
}

/** Branch and bound completo con las decisiones correctas, resolviendo cada nodo con el núcleo. */
function fullBnb(model: LpModel, tol: number, upFirst: boolean, pickLast: boolean): number | null {
  const root = model.variables.map(P.effBounds);
  const solveNode = (bounds: [number, number][]) => {
    const rep = solve(P.modelWithBounds(model, bounds), { relax_integers: true, diagnose: false });
    return { status: (rep.status === 'optimal' ? 'optimal' : rep.status === 'unbounded' ? 'unbounded' : 'infeasible') as P.NodeStatus, z: rep.objective_value, values: rep.variables.map((x) => x.value) };
  };
  let id = 0;
  const stack: P.BNode[] = [{ id: ++id, parent: null, depth: 0, branch: null, bounds: root, ...solveNode(root) }];
  let inc: number | null = null;
  let guard = 0;
  while (stack.length && guard++ < 200) {
    const n = stack.pop()!;
    const d = P.correctDecision(model, n, inc, tol).decision;
    ok(P.checkDecision(model, n, inc, tol, d).ok, 'la decisión correcta se acepta');
    if (d === 'incumbent') inc = n.z;
    if (d !== 'branch') continue;
    const fr = P.fractionalVars(model, n.values, tol);
    const j = pickLast ? fr[fr.length - 1] : fr[0];
    ok(P.checkBranchVar(model, n, j, tol).ok, 'variable de ramificación válida');
    const { down, up } = P.childBounds(n.bounds, j, n.values[j]);
    const mk = (b: [number, number][]): P.BNode => ({ id: ++id, parent: n.id, depth: n.depth + 1, branch: '', bounds: b, ...(b[j][0] > b[j][1] ? { status: 'infeasible' as const, z: null, values: [] } : solveNode(b)) });
    const a = mk(down);
    const b = mk(up);
    if (upFirst) stack.push(a, b);
    else stack.push(b, a);
  }
  return inc;
}

section('Branch and bound completo contra el núcleo');
{
  const ejemplos: LpModel[] = [
    {
      name: 'Entero',
      objective: 'max',
      variables: [v('X1', 5, { type: 'integer' }), v('X2', 8, { type: 'integer' })],
      constraints: [c('C1', [1, 1], '<=', 6), c('C2', [5, 9], '<=', 45)]
    },
    {
      name: 'Mochila',
      objective: 'max',
      variables: [v('A', 16, { type: 'binary', upper: 1 }), v('B', 22, { type: 'binary', upper: 1 }), v('C', 12, { type: 'binary', upper: 1 }), v('D', 8, { type: 'binary', upper: 1 })],
      constraints: [c('Peso', [5, 7, 4, 3], '<=', 14)]
    },
    {
      name: 'Mínimo mixto',
      objective: 'min',
      variables: [v('X1', 3, { type: 'integer' }), v('X2', 2), v('X3', 4, { type: 'integer' })],
      constraints: [c('C1', [2, 1, 3], '>=', 7.5), c('C2', [1, 2, 1], '>=', 5.2)]
    },
    {
      name: 'Entero infactible',
      objective: 'max',
      variables: [v('X1', 1, { type: 'integer' })],
      constraints: [c('C1', [2], '>=', 3), c('C2', [2], '<=', 3.5)]
    }
  ];
  for (const m of ejemplos) {
    const ref = solve(m, { bnb_trace: true, diagnose: false });
    for (const [upFirst, pickLast] of [
      [false, false],
      [true, false],
      [false, true],
      [true, true]
    ]) {
      const z = fullBnb(m, 0.01, upFirst, pickLast);
      if (ref.status === 'optimal') ok(P.sameObjective(z, ref.objective_value), `${m.name}: z = ${z} y el núcleo da ${ref.objective_value}`);
      else ok(z === null && ref.status === 'infeasible', `${m.name}: sin solución entera (${ref.status})`);
    }
  }
}

console.log(`\n${passes} comprobaciones correctas, ${fails} fallas.`);
process.exit(fails ? 1 : 0);
