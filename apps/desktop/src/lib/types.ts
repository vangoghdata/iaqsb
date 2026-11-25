// Tipos que reflejan el JSON de iaqsb-api (módulo LP-ILP).

export type Bound = number | 'M' | '-M';
export type VarType = 'continuous' | 'integer' | 'binary' | 'unrestricted' | 'semicontinuous' | 'semiinteger';
export type Direction = '<=' | '>=' | '=';
export type ObjectiveSense = 'max' | 'min';

export interface Variable {
  name: string;
  cost: number;
  lower?: Bound;
  upper?: Bound;
  type?: VarType;
  priority?: number;
}

export interface Constraint {
  name: string;
  coefs: number[];
  direction: Direction;
  rhs: number;
}

export interface LpModel {
  name: string;
  objective: ObjectiveSense;
  variables: Variable[];
  constraints: Constraint[];
}

export type Status = 'optimal' | 'infeasible' | 'unbounded' | 'iteration_limit' | 'node_limit' | 'feasible' | 'time_limit' | 'error';

/** Lo que hizo el motor de optimización (HiGHS, Gurobi, CPLEX…). */
export interface EngineInfo {
  solver: string;
  status?: string;
  time?: number;
  objective?: number | null;
  bound?: number | null;
  gap?: number | null;
  nodes?: number;
  iterations?: number;
  cols?: number;
  rows?: number;
  nonzeros?: number;
  is_mip?: boolean;
  /** Módulos genéricos: cuántas veces se llamó al motor y si alguna se detuvo por un límite. */
  runs?: number;
  limited?: boolean;
}

/** Por qué un modelo no es factible. */
export interface Diagnosis {
  conflict: { kind: string; name: string; text: string }[];
  relaxation: { kind: string; name: string; change: number; text: string }[];
  relaxation_total: number | null;
  summary: string;
}

export interface VariableResult {
  name: string;
  value: number;
  cost: number;
  contribution: number;
  reduced_cost: number;
  basis_status: 'basic' | 'at_bound';
  cost_min: Bound | null;
  cost_max: Bound | null;
}

export interface ConstraintResult {
  name: string;
  lhs: number;
  direction: Direction;
  rhs: number;
  slack_or_surplus: number;
  shadow_price: number | null;
  rhs_min: Bound | null;
  rhs_max: Bound | null;
}

export interface MValue {
  m: number;
  c: number;
}

export interface TableauSnapshot {
  iteration: number;
  columns: string[];
  cj: MValue[];
  basis: string[];
  cb: MValue[];
  rows: number[][];
  rhs: number[];
  cj_minus_zj: MValue[];
  objective: MValue;
  entering: string | null;
  leaving: string | null;
}

export interface BnbNode {
  node: number;
  depth: number;
  branches: string[];
  status: Status;
  objective_value: number | null;
  values: number[];
  action: string;
}

export interface LpReport {
  problem: string;
  objective: ObjectiveSense;
  status: Status;
  method: 'simplex' | 'branch_and_bound' | 'branch_and_cut' | 'engine' | string;
  objective_value: number | null;
  iterations: number;
  variables: VariableResult[];
  constraints: ConstraintResult[];
  alternate_optima: boolean;
  sensitivity: boolean;
  unbounded_variable?: string;
  message: string;
  simplex_trace?: TableauSnapshot[];
  branch_and_bound?: {
    relaxed_objective: number | null;
    nodes_explored: number;
    incumbents_found: number;
    trace?: BnbNode[];
  };
  /** Sección «Fuentes»: métodos citados y origen de los datos. */
  sections?: { id: string; title: string; blocks: import('./gtypes').SourcesBlock[] }[];
  engine?: EngineInfo;
  diagnosis?: Diagnosis;
  objective_values?: [string, number][];
  log?: string;
  notes?: string[];
}

export interface GraphicData {
  x_name: string;
  y_name: string;
  x_range: [number, number];
  y_range: [number, number];
  feasible_region: [number, number][];
  region_unbounded: boolean;
  lines: { name: string; a: number; b: number; direction: Direction; rhs: number }[];
  optimum: [number, number] | null;
  objective_value: number | null;
  objective_coefs: [number, number];
  status: Status;
}

export interface ModuleInfo {
  id: string;
  winqsb: string;
  name: string;
  solves: string;
  file_ext: string;
  available: boolean;
  /** Tema de un curso instalado (.ncourse): id c.<curso>.<tema>. */
  curso?: { id: string; titulo: string };
  /** El tema es solo lección (no calcula). */
  leccion?: boolean;
  /** Tema de curso basado en un módulo de Nativo: el id del módulo. */
  base?: string | null;
}
