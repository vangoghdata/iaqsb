//! Libros de Excel (.xlsx) sin dependencias pesadas: escribir hojas con números
//! y textos (encabezado en negrita y fijo) y leer los valores de cualquier libro
//! (Excel, LibreOffice, Google Sheets). Fórmulas: se lee el último valor calculado.

use crate::zip::{self, ZipWriter};

#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Empty,
    Num(f64),
    Text(String),
    Bool(bool),
}

impl Cell {
    pub fn text(&self) -> String {
        match self {
            Cell::Empty => String::new(),
            Cell::Num(x) => fmt_num(*x),
            Cell::Text(s) => s.clone(),
            Cell::Bool(b) => if *b { "TRUE".into() } else { "FALSE".into() },
        }
    }
    pub fn num(&self) -> Option<f64> {
        match self {
            Cell::Num(x) => Some(*x),
            Cell::Text(s) => parse_num(s),
            Cell::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            Cell::Empty => None,
        }
    }
    pub fn is_empty(&self) -> bool {
        match self {
            Cell::Empty => true,
            Cell::Text(s) => s.trim().is_empty(),
            _ => false,
        }
    }
}

impl From<f64> for Cell {
    fn from(x: f64) -> Self {
        Cell::Num(x)
    }
}
impl From<&str> for Cell {
    fn from(s: &str) -> Self {
        Cell::Text(s.into())
    }
}
impl From<String> for Cell {
    fn from(s: String) -> Self {
        Cell::Text(s)
    }
}

/// Número escrito con punto o coma decimal («1.234,5», «1,234.5», «3,5», «1e-3», «12 %»).
pub fn parse_num(s: &str) -> Option<f64> {
    let t = s.trim().trim_end_matches('%').trim().replace([' ', '\u{a0}', '$'], "");
    if t.is_empty() {
        return None;
    }
    if let Ok(x) = t.parse::<f64>() {
        return Some(x);
    }
    let (c, d) = (t.matches(',').count(), t.matches('.').count());
    let fixed = if c > 0 && d > 0 {
        // el último separador es el decimal
        if t.rfind(',') > t.rfind('.') { t.replace('.', "").replace(',', ".") } else { t.replace(',', "") }
    } else if c == 1 {
        t.replace(',', ".")
    } else if c > 1 {
        t.replace(',', "")
    } else {
        t
    };
    fixed.parse::<f64>().ok()
}

fn fmt_num(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        let s = format!("{x}");
        s
    }
}

/// Una hoja del libro.
#[derive(Debug, Clone, Default)]
pub struct Sheet {
    pub name: String,
    pub rows: Vec<Vec<Cell>>,
    /// Filas de encabezado (en negrita y fijas al desplazarse).
    pub header_rows: usize,
}

impl Sheet {
    pub fn new(name: &str) -> Self {
        Sheet { name: name.into(), rows: vec![], header_rows: 0 }
    }
    pub fn cell(&self, r: usize, c: usize) -> &Cell {
        self.rows.get(r).and_then(|row| row.get(c)).unwrap_or(&Cell::Empty)
    }
    /// Filas sin las vacías del final y sin celdas vacías al final de cada fila.
    pub fn trimmed(&self) -> Vec<Vec<Cell>> {
        let mut rows: Vec<Vec<Cell>> = self
            .rows
            .iter()
            .map(|r| {
                let mut r = r.clone();
                while r.last().is_some_and(|c| c.is_empty()) {
                    r.pop();
                }
                r
            })
            .collect();
        while rows.last().is_some_and(|r| r.is_empty()) {
            rows.pop();
        }
        rows
    }
}

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            // caracteres de control que XML no admite
            c if (c as u32) < 0x20 && c != '\n' && c != '\t' && c != '\r' => {}
            c => o.push(c),
        }
    }
    o
}

/// Columna de Excel: 0 → A, 25 → Z, 26 → AA.
pub fn col_name(mut c: usize) -> String {
    let mut s = vec![];
    loop {
        s.push((b'A' + (c % 26) as u8) as char);
        if c < 26 {
            break;
        }
        c = c / 26 - 1;
    }
    s.iter().rev().collect()
}

fn col_index(letters: &str) -> usize {
    letters.bytes().filter(|b| b.is_ascii_alphabetic()).fold(0usize, |acc, b| acc * 26 + (b.to_ascii_uppercase() - b'A' + 1) as usize) - 1
}

/// Nombre de hoja válido para Excel (31 caracteres, sin : \ / ? * [ ]) y sin repetir.
fn sheet_names(sheets: &[Sheet]) -> Vec<String> {
    let mut used: Vec<String> = vec![];
    for s in sheets {
        let mut base: String = s.name.chars().filter(|c| !":\\/?*[]".contains(*c)).collect::<String>().trim().to_string();
        if base.is_empty() {
            base = format!("Hoja{}", used.len() + 1);
        }
        let base: String = base.chars().take(31).collect();
        let mut name = base.clone();
        let mut k = 2;
        while used.iter().any(|u| u.eq_ignore_ascii_case(&name)) {
            let suf = format!(" ({k})");
            name = format!("{}{suf}", base.chars().take(31 - suf.chars().count()).collect::<String>());
            k += 1;
        }
        used.push(name);
    }
    used
}

/// Escribe un libro .xlsx.
pub fn write(sheets: &[Sheet]) -> Vec<u8> {
    let names = sheet_names(sheets);
    let mut z = ZipWriter::new();
    let mut ct = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/><Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>"#);
    for i in 0..sheets.len() {
        ct.push_str(&format!(r#"<Override PartName="/xl/worksheets/sheet{}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#, i + 1));
    }
    ct.push_str("</Types>");
    z.add("[Content_Types].xml", ct.as_bytes());
    z.add("_rels/.rels", br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>"#);
    z.add("docProps/core.xml", br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:creator>IAQSB</dc:creator></cp:coreProperties>"#);
    z.add("docProps/app.xml", br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>IAQSB</Application></Properties>"#);
    let mut wb = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>"#);
    let mut rels = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#);
    for (i, n) in names.iter().enumerate() {
        wb.push_str(&format!(r#"<sheet name="{}" sheetId="{}" r:id="rId{}"/>"#, esc(n), i + 1, i + 1));
        rels.push_str(&format!(r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{}.xml"/>"#, i + 1, i + 1));
    }
    wb.push_str("</sheets></workbook>");
    rels.push_str(&format!(r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#, sheets.len() + 1));
    z.add("xl/workbook.xml", wb.as_bytes());
    z.add("xl/_rels/workbook.xml.rels", rels.as_bytes());
    // estilos: 0 normal, 1 encabezado (negrita, fondo gris claro, borde inferior)
    z.add("xl/styles.xml", br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><name val="Calibri"/></font></fonts><fills count="3"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FFF2F2F7"/></patternFill></fill></fills><borders count="2"><border><left/><right/><top/><bottom/><diagonal/></border><border><left/><right/><top/><bottom style="thin"><color rgb="FF8E8E93"/></bottom><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="2"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/><xf numFmtId="0" fontId="1" fillId="2" borderId="1" xfId="0" applyFont="1" applyFill="1" applyBorder="1"/></cellXfs></styleSheet>"#);
    for (i, sh) in sheets.iter().enumerate() {
        let ncols = sh.rows.iter().map(|r| r.len()).max().unwrap_or(0);
        let mut x = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#);
        if sh.header_rows > 0 {
            x.push_str(&format!(r#"<sheetViews><sheetView workbookViewId="0"><pane ySplit="{0}" topLeftCell="A{1}" activePane="bottomLeft" state="frozen"/></sheetView></sheetViews>"#, sh.header_rows, sh.header_rows + 1));
        }
        // ancho de cada columna según su texto más largo
        if ncols > 0 {
            x.push_str("<cols>");
            for c in 0..ncols {
                let w = sh.rows.iter().filter_map(|r| r.get(c)).map(|v| v.text().chars().count()).max().unwrap_or(8).clamp(6, 60) as f64 + 2.0;
                x.push_str(&format!(r#"<col min="{0}" max="{0}" width="{w}" customWidth="1"/>"#, c + 1));
            }
            x.push_str("</cols>");
        }
        x.push_str("<sheetData>");
        for (r, row) in sh.rows.iter().enumerate() {
            x.push_str(&format!(r#"<row r="{}">"#, r + 1));
            let st = if r < sh.header_rows { r#" s="1""# } else { "" };
            for (c, v) in row.iter().enumerate() {
                let rf = format!("{}{}", col_name(c), r + 1);
                match v {
                    Cell::Empty if st.is_empty() => {}
                    Cell::Empty => x.push_str(&format!(r#"<c r="{rf}"{st}/>"#)),
                    Cell::Num(n) if n.is_finite() => x.push_str(&format!(r#"<c r="{rf}"{st}><v>{n}</v></c>"#)),
                    Cell::Num(n) => x.push_str(&format!(r#"<c r="{rf}"{st} t="inlineStr"><is><t>{}</t></is></c>"#, if *n > 0.0 { "M" } else if *n < 0.0 { "-M" } else { "" })),
                    Cell::Bool(b) => x.push_str(&format!(r#"<c r="{rf}"{st} t="b"><v>{}</v></c>"#, *b as u8)),
                    Cell::Text(t) => x.push_str(&format!(r#"<c r="{rf}"{st} t="inlineStr"><is><t xml:space="preserve">{}</t></is></c>"#, esc(t))),
                }
            }
            x.push_str("</row>");
        }
        x.push_str("</sheetData></worksheet>");
        z.add(&format!("xl/worksheets/sheet{}.xml", i + 1), x.as_bytes());
    }
    z.finish()
}

/// Lee las hojas de un libro .xlsx (valores; las fórmulas dan su último resultado).
pub fn read(bytes: &[u8]) -> Result<Vec<Sheet>, String> {
    if !zip::is_zip(bytes) {
        return Err("no es un libro de Excel (.xlsx)".into());
    }
    let files = zip::read(bytes)?;
    let get = |name: &str| files.iter().find(|(n, _)| n.trim_start_matches('/').eq_ignore_ascii_case(name)).map(|(_, d)| String::from_utf8_lossy(d).into_owned());
    let wb = get("xl/workbook.xml").ok_or("el archivo no tiene xl/workbook.xml (¿es un .xlsx?)")?;
    let rels = get("xl/_rels/workbook.xml.rels").unwrap_or_default();
    // textos compartidos
    let mut shared: Vec<String> = vec![];
    if let Some(ss) = get("xl/sharedStrings.xml") {
        let doc = roxmltree::Document::parse(&ss).map_err(|e| format!("sharedStrings.xml: {e}"))?;
        for si in doc.descendants().filter(|n| n.has_tag_name("si")) {
            shared.push(rich_text(si));
        }
    }
    let rel_doc = roxmltree::Document::parse(if rels.is_empty() { "<r/>" } else { &rels }).map_err(|e| format!("workbook.xml.rels: {e}"))?;
    let target = |id: &str| -> Option<String> {
        rel_doc.descendants().find(|n| n.has_tag_name("Relationship") && n.attribute("Id") == Some(id)).and_then(|n| n.attribute("Target")).map(|t| {
            let t = t.trim_start_matches('/');
            if t.starts_with("xl/") { t.to_string() } else { format!("xl/{t}") }
        })
    };
    let wdoc = roxmltree::Document::parse(&wb).map_err(|e| format!("workbook.xml: {e}"))?;
    let mut out = vec![];
    for (k, sh) in wdoc.descendants().filter(|n| n.has_tag_name("sheet")).enumerate() {
        let name = sh.attribute("name").unwrap_or("").to_string();
        let rid = sh.attributes().find(|a| a.name() == "id").map(|a| a.value().to_string()).unwrap_or_default();
        let path = target(&rid).unwrap_or_else(|| format!("xl/worksheets/sheet{}.xml", k + 1));
        let Some(xml) = get(&path) else { continue };
        let doc = roxmltree::Document::parse(&xml).map_err(|e| format!("{path}: {e}"))?;
        let mut rows: Vec<Vec<Cell>> = vec![];
        let mut next_row = 0usize;
        for row in doc.descendants().filter(|n| n.has_tag_name("row")) {
            let r = row.attribute("r").and_then(|x| x.parse::<usize>().ok()).map(|x| x - 1).unwrap_or(next_row);
            next_row = r + 1;
            if r > 1_000_000 {
                break;
            }
            while rows.len() <= r {
                rows.push(vec![]);
            }
            let mut next_col = 0usize;
            for c in row.children().filter(|n| n.has_tag_name("c")) {
                let col = c.attribute("r").map(col_index).unwrap_or(next_col);
                next_col = col + 1;
                if col > 16_384 {
                    continue;
                }
                let t = c.attribute("t").unwrap_or("n");
                let v = c.children().find(|n| n.has_tag_name("v")).and_then(|n| n.text()).unwrap_or("");
                let cell = match t {
                    "s" => v.trim().parse::<usize>().ok().and_then(|i| shared.get(i)).map(|s| Cell::Text(s.clone())).unwrap_or(Cell::Empty),
                    "inlineStr" => c.children().find(|n| n.has_tag_name("is")).map(|is| Cell::Text(rich_text(is))).unwrap_or(Cell::Empty),
                    "str" => Cell::Text(v.to_string()),
                    "b" => Cell::Bool(v.trim() == "1"),
                    "e" => Cell::Text(v.to_string()),
                    _ if v.trim().is_empty() => Cell::Empty,
                    _ => v.trim().parse::<f64>().map(Cell::Num).unwrap_or_else(|_| Cell::Text(v.to_string())),
                };
                let rr = &mut rows[r];
                while rr.len() <= col {
                    rr.push(Cell::Empty);
                }
                rr[col] = cell;
            }
        }
        out.push(Sheet { name, rows, header_rows: 0 });
    }
    if out.is_empty() {
        return Err("el libro no tiene hojas".into());
    }
    Ok(out)
}

/// Texto de un <si> o <is> (con formato enriquecido: varios <r><t>).
fn rich_text(n: roxmltree::Node) -> String {
    n.descendants().filter(|x| x.has_tag_name("t") && !x.ancestors().any(|a| a.has_tag_name("rPh"))).filter_map(|x| x.text()).collect()
}

/// Hojas como texto separado por tabuladores (lo que se pega desde Excel), una por hoja.
pub fn to_tsv(sheet: &Sheet) -> String {
    sheet.trimmed().iter().map(|r| r.iter().map(|c| c.text()).collect::<Vec<_>>().join("\t")).collect::<Vec<_>>().join("\n")
}

/// Texto pegado desde Excel, Google Sheets o LibreOffice (tabuladores; o ; o , si no hay)
/// como filas de celdas. Respeta las comillas de las celdas con saltos de línea.
pub fn parse_pasted(text: &str) -> Vec<Vec<Cell>> {
    let text = text.trim_end_matches(['\n', '\r']);
    let sep = if text.contains('\t') {
        '\t'
    } else if text.lines().all(|l| l.contains(';')) {
        ';'
    } else if text.lines().count() > 1 && text.lines().all(|l| l.contains(',')) && !text.contains(", ") {
        ','
    } else {
        '\t'
    };
    let mut rows = vec![];
    let mut row = vec![];
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    let push = |cur: &mut String, row: &mut Vec<Cell>| {
        let s = std::mem::take(cur);
        let t = s.trim();
        row.push(if t.is_empty() { Cell::Empty } else if let Some(x) = parse_num(t) { Cell::Num(x) } else { Cell::Text(t.to_string()) });
    };
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' if cur.trim().is_empty() && !quoted => quoted = true,
            '"' if quoted => quoted = false,
            c if c == sep && !quoted => push(&mut cur, &mut row),
            '\r' if !quoted => {}
            '\n' if !quoted => {
                push(&mut cur, &mut row);
                rows.push(std::mem::take(&mut row));
            }
            c => cur.push(c),
        }
    }
    push(&mut cur, &mut row);
    rows.push(row);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escribe_y_lee() {
        let mut s = Sheet::new("Modelo: LP/1");
        s.header_rows = 1;
        s.rows.push(vec!["Variable".into(), "X1".into(), "X2".into()]);
        s.rows.push(vec!["Max".into(), 3.0.into(), 5.0.into()]);
        s.rows.push(vec!["C1 <tope> & \"más\"".into(), Cell::Empty, Cell::Num(f64::INFINITY)]);
        let b = write(&[s.clone(), Sheet::new("Modelo: LP/1")]);
        let back = read(&b).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].name, "Modelo LP1");
        assert_ne!(back[0].name, back[1].name);
        assert_eq!(back[0].cell(1, 2), &Cell::Num(5.0));
        assert_eq!(back[0].cell(2, 0).text(), "C1 <tope> & \"más\"");
        assert_eq!(back[0].cell(2, 2).text(), "M");
        assert_eq!(col_name(0), "A");
        assert_eq!(col_name(27), "AB");
        assert_eq!(col_index("AB12"), 27);
    }

    #[test]
    fn numeros_y_pegado() {
        assert_eq!(parse_num("3,5"), Some(3.5));
        assert_eq!(parse_num("1.234,5"), Some(1234.5));
        assert_eq!(parse_num("1,234.5"), Some(1234.5));
        assert_eq!(parse_num("12 %"), Some(12.0));
        assert_eq!(parse_num("abc"), None);
        let p = parse_pasted("a\tb\n1,5\t2\n\"x\ny\"\t3\n");
        assert_eq!(p.len(), 3);
        assert_eq!(p[1][0], Cell::Num(1.5));
        assert_eq!(p[2][0], Cell::Text("x\ny".into()));
        let q = parse_pasted("1;2;3\n4;5;6");
        assert_eq!(q[1][2], Cell::Num(6.0));
    }
}
