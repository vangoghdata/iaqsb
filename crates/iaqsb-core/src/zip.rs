//! Archivos ZIP mínimos (lo que usan .docx y .xlsx): escribir con DEFLATE y leer
//! entradas guardadas o comprimidas. Sin ZIP64 (hasta 4 GB, de sobra aquí).

use std::io::{Read, Write};

/// Escritor de ZIP en memoria.
#[derive(Default)]
pub struct ZipWriter {
    out: Vec<u8>,
    central: Vec<u8>,
    count: u16,
}

impl ZipWriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Agrega un archivo; los textos (XML) se comprimen, las imágenes PNG no.
    pub fn add(&mut self, name: &str, data: &[u8]) {
        let compress = !name.ends_with(".png") && data.len() > 64;
        let crc = crc32fast::hash(data);
        let (method, body): (u16, Vec<u8>) = if compress {
            let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            e.write_all(data).expect("compresión en memoria");
            (8, e.finish().expect("compresión en memoria"))
        } else {
            (0, data.to_vec())
        };
        let offset = self.out.len() as u32;
        let name_b = name.as_bytes();
        // fecha fija (1 de enero de 2026): el archivo sale igual cada vez
        let (time, date): (u16, u16) = (0, ((2026 - 1980) << 9 | 1 << 5 | 1) as u16);
        let w16 = |v: &mut Vec<u8>, x: u16| v.extend_from_slice(&x.to_le_bytes());
        let w32 = |v: &mut Vec<u8>, x: u32| v.extend_from_slice(&x.to_le_bytes());
        // cabecera local
        let o = &mut self.out;
        w32(o, 0x0403_4b50);
        w16(o, 20);
        w16(o, 0x0800); // nombres en UTF-8
        w16(o, method);
        w16(o, time);
        w16(o, date);
        w32(o, crc);
        w32(o, body.len() as u32);
        w32(o, data.len() as u32);
        w16(o, name_b.len() as u16);
        w16(o, 0);
        o.extend_from_slice(name_b);
        o.extend_from_slice(&body);
        // directorio central
        let c = &mut self.central;
        w32(c, 0x0201_4b50);
        w16(c, 20);
        w16(c, 20);
        w16(c, 0x0800);
        w16(c, method);
        w16(c, time);
        w16(c, date);
        w32(c, crc);
        w32(c, body.len() as u32);
        w32(c, data.len() as u32);
        w16(c, name_b.len() as u16);
        w16(c, 0);
        w16(c, 0);
        w16(c, 0);
        w16(c, 0);
        w32(c, 0);
        w32(c, offset);
        c.extend_from_slice(name_b);
        self.count += 1;
    }

    pub fn finish(mut self) -> Vec<u8> {
        let start = self.out.len() as u32;
        let size = self.central.len() as u32;
        self.out.extend_from_slice(&self.central);
        let o = &mut self.out;
        o.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        o.extend_from_slice(&[0, 0, 0, 0]);
        o.extend_from_slice(&self.count.to_le_bytes());
        o.extend_from_slice(&self.count.to_le_bytes());
        o.extend_from_slice(&size.to_le_bytes());
        o.extend_from_slice(&start.to_le_bytes());
        o.extend_from_slice(&[0, 0]);
        self.out
    }
}

/// ¿Empieza como un ZIP?
pub fn is_zip(bytes: &[u8]) -> bool {
    bytes.len() > 22 && bytes.starts_with(b"PK\x03\x04")
}

/// Lee todas las entradas de un ZIP: (nombre, contenido).
pub fn read(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let r16 = |p: usize| -> Result<u16, String> { bytes.get(p..p + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or_else(|| "ZIP cortado".to_string()) };
    let r32 = |p: usize| -> Result<u32, String> { bytes.get(p..p + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(|| "ZIP cortado".to_string()) };
    // fin del directorio central (al final, quizá después de un comentario)
    let min = bytes.len().saturating_sub(22 + 65_535);
    let eocd = (min..=bytes.len().saturating_sub(22)).rev().find(|&p| bytes[p..].starts_with(b"PK\x05\x06")).ok_or("no es un archivo ZIP válido")?;
    let n = r16(eocd + 10)? as usize;
    let mut p = r32(eocd + 16)? as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        if r32(p)? != 0x0201_4b50 {
            return Err("directorio del ZIP dañado".into());
        }
        let method = r16(p + 10)?;
        let csize = r32(p + 20)? as usize;
        let usize_ = r32(p + 24)? as usize;
        let nlen = r16(p + 28)? as usize;
        let elen = r16(p + 30)? as usize;
        let clen = r16(p + 32)? as usize;
        let local = r32(p + 42)? as usize;
        let name = String::from_utf8_lossy(bytes.get(p + 46..p + 46 + nlen).ok_or("ZIP cortado")?).into_owned();
        p += 46 + nlen + elen + clen;
        let lnlen = r16(local + 26)? as usize;
        let lelen = r16(local + 28)? as usize;
        let start = local + 30 + lnlen + lelen;
        let raw = bytes.get(start..start + csize).ok_or("ZIP cortado")?;
        let data = match method {
            0 => raw.to_vec(),
            8 => {
                let mut v = Vec::with_capacity(usize_);
                flate2::read::DeflateDecoder::new(raw).read_to_end(&mut v).map_err(|e| format!("no se pudo descomprimir {name}: {e}"))?;
                v
            }
            m => return Err(format!("{name}: compresión {m} no admitida")),
        };
        out.push((name, data));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_y_vuelta() {
        let mut z = ZipWriter::new();
        let texto = "<a>".repeat(500);
        z.add("xl/workbook.xml", texto.as_bytes());
        z.add("img.png", &[1, 2, 3]);
        z.add("vacío.txt", b"");
        let b = z.finish();
        assert!(is_zip(&b));
        let e = read(&b).unwrap();
        assert_eq!(e.len(), 3);
        assert_eq!(e[0].1, texto.as_bytes());
        assert_eq!(e[1].1, vec![1, 2, 3]);
        assert_eq!(e[2].0, "vacío.txt");
    }
}
