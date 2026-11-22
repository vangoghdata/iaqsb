// Prueba de punta a punta (modo navegador) de la integración con Excel y de la
// exportación de informes: pegar desde Excel en la hoja de LP-ILP y en una
// matriz de un módulo genérico, exportar el informe en los cinco formatos,
// copiar una tabla de resultados y la ida y vuelta del modelo por .xlsx.
//
// Uso:
//   npm run build
//   python3 tools/dev_server.py ../../target/debug/iaqsb 1491 &
//   node tools/test_excel_ui.mjs [http://127.0.0.1:1491] [carpeta de salida]
//
// Necesita Playwright (npm i -D playwright, o PLAYWRIGHT_PATH=/ruta/a/playwright).

import { createRequire } from 'node:module';
import { mkdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const require = createRequire(import.meta.url);
let pw;
try {
  pw = require('playwright');
} catch {
  pw = require(process.env.PLAYWRIGHT_PATH ?? '/opt/npm-tools/node_modules/playwright');
}
const URL = process.argv[2] ?? 'http://127.0.0.1:1491/';
const OUT = process.argv[3] ?? join(process.cwd(), 'test-excel-salida');
mkdirSync(OUT, { recursive: true });

let fails = 0;
let passes = 0;
function ok(cond, what) {
  if (cond) passes++;
  else {
    fails++;
    console.error('  FALLA: ' + what);
  }
}

const browser = await pw.chromium.launch();
const ctx = await browser.newContext({ viewport: { width: 1400, height: 900 }, locale: 'es-CO', acceptDownloads: true });
await ctx.grantPermissions(['clipboard-read', 'clipboard-write'], { origin: new globalThis.URL(URL).origin });
const page = await ctx.newPage();
const errores = [];
page.on('pageerror', (e) => errores.push(e.message));
await page.addInitScript(() => {
  localStorage.setItem('iaqsb_menu_fijo', '1');
  localStorage.setItem('iaqsb_idioma', 'es');
});
await page.goto(URL);
await page.waitForSelector('[data-r="1"][data-c="1"]');

/** Pega `texto` como lo haría Excel (evento paste con text/plain) en la celda `sel`. */
async function pegar(sel, texto) {
  await page.focus(sel);
  await page.evaluate(
    ({ sel, texto }) => {
      const el = document.querySelector(sel);
      const dt = new DataTransfer();
      dt.setData('text/plain', texto);
      el.dispatchEvent(new ClipboardEvent('paste', { clipboardData: dt, bubbles: true, cancelable: true }));
    },
    { sel, texto }
  );
}
const valor = (sel) => page.$eval(sel, (el) => el.value);
async function aviso(re) {
  const t = page.locator('.avisos, [class*="aviso"]').filter({ hasText: re });
  try {
    await t.first().waitFor({ timeout: 3000 });
    return true;
  } catch {
    return (await page.content()).match(re) !== null;
  }
}

// ------------------------------------------------------------ pegar en la hoja de LP-ILP
console.log('· pegar en LP-ILP');
await pegar('[data-r="2"][data-c="1"]', '1,5\t2,5\r\n3\t4\r\n');
await page.waitForTimeout(200);
ok((await valor('[data-r="2"][data-c="1"]')) === '1.5', 'coma decimal 1,5 → 1.5');
ok((await valor('[data-r="3"][data-c="2"]')) === '4', 'bloque 2 × 2');
ok(await aviso(/Se pegaron 2 × 2 celdas/), 'aviso del pegado');

// la tabla completa (encabezado, nombres, sentido, lado derecho): falta una restricción
const wyndor = '\tPuertas\tVentanas\t\tLD\r\nMax\t3\t5\t\t\r\nPlanta1\t1\t0\t<=\t4\r\nPlanta2\t0\t2\t<=\t12\r\nPlanta3\t3\t2\t<=\t18\r\n';
await pegar('[data-r="1"][data-c="1"]', wyndor);
await page.waitForSelector('.op.principal');
await page.screenshot({ path: join(OUT, 'pegar_dialogo.png') });
await page.click('.op.principal');
await page.waitForTimeout(150);
await page.screenshot({ path: join(OUT, 'pegar_resaltado.png') });
ok((await valor('[data-r="0"][data-c="1"]')) === 'Puertas', 'renombra X1');
ok((await valor('[data-r="4"][data-c="0"]')) === 'Planta3', 'añade la restricción Planta3');
ok((await valor('[data-r="4"][data-c="5"]')) === '18', 'lado derecho de Planta3');
ok((await valor('[data-r="1"][data-c="2"]')) === '5', 'costo de Ventanas');

// ------------------------------------------------------------ resolver y exportar
console.log('· resolver y exportar');
await page.click('button.solve');
await page.waitForSelector('.zval');
ok((await page.textContent('.zval')).trim() === '36', 'z = 36');

const firmas = { docx: 'PK', xlsx: 'PK', pdf: '%PDF', html: '<!doctype html', md: '##' };
for (const f of ['docx', 'pdf', 'html', 'xlsx', 'md']) {
  await page.click('.exp > .btn');
  await page.waitForSelector('.exp .panel');
  if (f === 'docx') {
    await page.fill('.exp .portada input >> nth=0', 'Ana Pérez');
    await page.fill('.exp .portada input >> nth=1', 'Investigación de operaciones I');
    await page.fill('.exp .portada input >> nth=2', 'Universidad Nacional');
    await page.screenshot({ path: join(OUT, 'exportar_menu.png') });
  }
  await page.click(`.exp .fmt:has(.b-${f})`);
  const [dl] = await Promise.all([page.waitForEvent('download'), page.click('.exp .ir')]);
  const ruta = join(OUT, `lp.${f}`);
  await dl.saveAs(ruta);
  const cab = readFileSync(ruta).subarray(0, 20).toString('latin1');
  ok(cab.toLowerCase().startsWith(firmas[f].toLowerCase()), `formato ${f}: empieza con ${firmas[f]} (${JSON.stringify(cab.slice(0, 8))})`);
  ok(dl.suggestedFilename().endsWith('.' + f), `nombre .${f}: ${dl.suggestedFilename()}`);
}
// la portada quedó en los ajustes
const aj = await page.evaluate(() => JSON.parse(localStorage.getItem('iaqsb.ajustes') ?? '{}'));
ok(aj.autor === 'Ana Pérez' && aj.institucion === 'Universidad Nacional', 'la portada se guarda en los ajustes');
const html = readFileSync(join(OUT, 'lp.html'), 'utf8');
ok(html.includes('Ana Pérez') && html.includes('Planta3'), 'el HTML trae la portada y el modelo');

// ------------------------------------------------------------ copiar una tabla de resultados
console.log('· copiar tabla');
await page.hover('.sol .tw >> nth=0');
await page.waitForSelector('.iaqsb-copiar.ver');
await page.screenshot({ path: join(OUT, 'copiar_tabla.png') });
await page.click('.iaqsb-copiar.ver');
await page.waitForTimeout(200);
const tsv = await page.evaluate(() => navigator.clipboard.readText());
ok(/Puertas\t2\b/.test(tsv), 'TSV con Puertas y su valor: ' + JSON.stringify(tsv.slice(0, 120)));
ok(tsv.split(/\r?\n/)[0].includes('\t'), 'encabezado separado por tabuladores');

// ------------------------------------------------------------ LP: ida y vuelta por Excel
console.log('· LP: Excel ida y vuelta');
await page.selectOption('select[aria-label="Formato del archivo"]', 'xlsx');
const [dlx] = await Promise.all([page.waitForEvent('download'), page.click('.savegrp .btn')]);
const rutaX = join(OUT, 'modelo_lp.xlsx');
await dlx.saveAs(rutaX);
ok(readFileSync(rutaX).subarray(0, 2).toString() === 'PK', 'modelo LP .xlsx');
// problema nuevo y se abre el libro
await page.click('button[aria-label="Nuevo problema"]');
await page.waitForTimeout(150);
const [fc] = await Promise.all([page.waitForEvent('filechooser'), page.click('button[aria-label="Abrir"]')]);
await fc.setFiles(rutaX);
await page.waitForFunction(() => document.querySelector('[data-r="0"][data-c="1"]')?.value === 'Puertas');
ok((await valor('[data-r="4"][data-c="5"]')) === '18', 'LP leído del .xlsx: lado derecho');
ok((await valor('[data-r="2"][data-c="0"]')) === 'Planta1', 'LP leído del .xlsx: nombres');

// ------------------------------------------------------------ módulo genérico: NET (transporte)
console.log('· NET: pegar en la matriz, exportar y Excel ida y vuelta');
await page.click('.item.mod:has(.wq:text-is("NET"))');
await page.waitForSelector('.grid [data-r="0"][data-c="0"]');
// problema de transporte nuevo, 2 × 2
await page.click('button[aria-label="Nuevo problema"]');
await page.click('dialog .types button:has-text("transporte")');
await page.click('dialog .actions .btn.primary');
await page.waitForTimeout(300);
const tabla = '\tBogotá\tCali\tMedellín\tOferta\r\nPlanta A\t4\t6\t9\t100\r\nPlanta B\t5\t3\t7\t150\r\nDemanda\t80\t70\t100\t\r\n';
await pegar('.grid [data-r="0"][data-c="0"]', tabla);
if (await page.$('.op.principal')) {
  await page.screenshot({ path: join(OUT, 'pegar_matriz_dialogo.png') });
  await page.click('.op.principal');
}
await page.waitForTimeout(200);
await page.screenshot({ path: join(OUT, 'pegar_matriz.png') });
const nombres = await page.$$eval('.grid input[data-hc]', (els) => els.map((e) => e.value));
ok(nombres.slice(0, 3).join(',') === 'Bogotá,Cali,Medellín', 'NET: destinos renombrados ' + nombres.join(','));
await page.click('button.solve');
await page.waitForSelector('.headline');
// el informe en Word (para revisar con LibreOffice)
await page.click('.exp > .btn');
await page.click('.exp .fmt:has(.b-docx)');
const [dln] = await Promise.all([page.waitForEvent('download'), page.click('.exp .ir')]);
await dln.saveAs(join(OUT, 'net.docx'));
ok(readFileSync(join(OUT, 'net.docx')).subarray(0, 2).toString() === 'PK', 'NET .docx');
// el modelo a Excel desde el menú Exportar
await page.click('.exp > .btn');
const [dlm] = await Promise.all([page.waitForEvent('download'), page.click('.exp .modelo .btn')]);
const rutaN = join(OUT, 'modelo_net.xlsx');
await dlm.saveAs(rutaN);
// otro problema y se abre el libro
await page.click('button[aria-label="Nuevo problema"]');
await page.click('dialog .types button:has-text("transporte")');
await page.click('dialog .actions .btn.primary');
await page.waitForTimeout(300);
const [fc2] = await Promise.all([page.waitForEvent('filechooser'), page.click('button[aria-label="Abrir"]')]);
await fc2.setFiles(rutaN);
await page.waitForFunction(() => document.querySelector('.grid input[data-hc="2"]')?.value === 'Medellín');
const costo = await page.$eval('.grid [data-r="1"][data-c="2"]', (e) => e.value);
ok(costo === '7', 'NET leído del .xlsx: costo Planta B → Medellín = 7');

ok(errores.length === 0, 'sin errores en la página: ' + errores.join(' | '));
await browser.close();
console.log(`${passes} bien, ${fails} fallas · archivos en ${OUT}`);
process.exit(fails ? 1 : 0);
