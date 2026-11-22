// Tipos de la especificación de módulos y de los informes genéricos (iaqsb-core).

export type FieldKind =
  | { type: 'number' }
  | { type: 'integer' }
  | { type: 'text' }
  | { type: 'bool' }
  | { type: 'choice'; options: [string, string][] }
  | { type: 'bound' }
  | { type: 'list' }
  | { type: 'expr' };

export interface Field {
  key: string;
  label: string;
  kind: FieldKind;
  help?: string;
  default?: unknown;
  show_if?: [string, string[]][];
  comparable?: boolean;
  optional?: boolean;
}

export interface Axis {
  path: string;
  label_key: string;
  label: string;
  extra: Field[];
  add_label?: string;
}

export type Widget =
  | { type: 'params'; title?: string; path?: string; fields: Field[] }
  | { type: 'records'; title?: string; path: string; columns: Field[]; add_label?: string }
  | { type: 'matrix'; title?: string; path: string; rows: Axis; cols: Axis; cell: Field; allow_diagonal?: boolean };

export interface ProblemType {
  id: string;
  label: string;
  description: string;
  dims: Field[];
  layout: Widget[];
}

export interface ModuleSpec {
  id: string;
  winqsb: string;
  name: string;
  description: string;
  file_ext: string;
  problem_types: ProblemType[];
  options: Field[];
}

export type Cell = number | string | null;
export type Tone = 'normal' | 'accent' | 'highlight' | 'good' | 'bad' | 'muted' | 'total';

export interface Metric {
  label: string;
  value: Cell;
  unit?: string;
  emphasis?: boolean;
}

export interface TableBlock {
  type: 'table';
  title?: string;
  columns: { label: string; align?: string }[];
  rows: { cells: Cell[]; tone?: Tone }[];
  marks?: { row: number; col: number; tone: Tone }[];
  col_marks?: [number, Tone][];
  note?: string;
}

export interface Series {
  name: string;
  kind: 'line' | 'bar' | 'scatter' | 'step' | 'area';
  tone?: Tone;
  dashed?: boolean;
  points: [number, number][];
}

export interface RefLine {
  value: number;
  label: string;
  tone?: Tone;
}

export interface ChartBlock {
  type: 'chart';
  title?: string;
  x_label: string;
  y_label: string;
  categories?: string[];
  series: Series[];
  hlines?: RefLine[];
  vlines?: RefLine[];
  y_zero?: boolean;
  stacked?: boolean;
}

export interface MapBlock {
  type: 'map';
  title?: string;
  points: { id: string; label: string; x: number; y: number; kind: 'depot' | 'stop' | string; note?: string; route?: number }[];
  routes: { name: string; stops: string[]; note?: string; dashed?: boolean }[];
  rays?: { from: string; angle: number; label?: string }[];
  arrows?: boolean;
  /** Coordenadas geográficas: x = longitud, y = latitud (en grados). */
  geo?: boolean;
}

export interface Citation {
  id: string;
  cite: string;
  apa: string;
  link?: string;
  verified: boolean;
  pending?: string;
  used_for: string[];
  bibtex: string;
}

export interface DataSource {
  key: string;
  origin: 'libro' | 'clase' | 'adaptado' | 'winqsb' | 'original' | string;
  origin_label: string;
  summary: string;
  refs: Citation[];
  expected?: string;
  note?: string;
  verified: boolean;
  pending?: string;
}

export interface LayoutBlock {
  type: 'layout';
  title?: string;
  width: number;
  height: number;
  unit: string;
  rects: { x: number; y: number; w: number; h: number; label?: string; tone?: Tone; soft?: boolean }[];
  dims?: { x1: number; y1: number; x2: number; y2: number; label: string }[];
}

export interface SourcesBlock {
  type: 'sources';
  data?: DataSource;
  refs: Citation[];
}

export interface NetworkBlock {
  type: 'network';
  title?: string;
  directed: boolean;
  nodes: { id: string; label: string; x: number; y: number; note?: string; tone?: Tone }[];
  edges: { from: string; to: string; label?: string; tone?: Tone }[];
}

export interface GanttBlock {
  type: 'gantt';
  title?: string;
  unit: string;
  rows: { label: string; bars: { start: number; end: number; label?: string; tone?: Tone }[] }[];
  markers?: RefLine[];
  legend?: [Tone, string][];
}

export interface TreeBlock {
  type: 'tree';
  title?: string;
  nodes: { id: number; parent?: number; edge?: string; label: string; kind?: string; value?: string; tone?: Tone }[];
}

export interface StepsBlock {
  type: 'steps';
  title?: string;
  steps: { title: string; description?: string; blocks: Block[] }[];
}

export type Block =
  | { type: 'metrics'; title?: string; items: Metric[] }
  | TableBlock
  | { type: 'text'; title?: string; body: string; tone?: Tone }
  | ChartBlock
  | NetworkBlock
  | GanttBlock
  | TreeBlock
  | MapBlock
  | SourcesBlock
  | LayoutBlock
  | StepsBlock;

export interface Report {
  module: string;
  problem: string;
  status: 'optimal' | 'feasible' | 'infeasible' | 'unbounded' | 'solved';
  headline: string;
  sections: { id: string; title: string; blocks: Block[] }[];
  data: unknown;
  references?: Citation[];
  data_source?: DataSource;
  engine?: import('./types').EngineInfo;
}

/** ¿Se cumplen todas las condiciones de visibilidad del campo? `get` busca cada clave. */
export function shown(f: { show_if?: [string, string[]][] }, get: (key: string) => unknown): boolean {
  return (f.show_if ?? []).every(([k, vals]) => vals.includes(String(get(k))));
}
