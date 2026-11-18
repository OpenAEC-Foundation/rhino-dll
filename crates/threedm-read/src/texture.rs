//! Grey values of a texture image, for displacement: PNG (8 and 16 bits, grey, grey and alpha,
//! RGB, RGBA and palette, interlaced too) and uncompressed BMP (8, 24 and 32 bits). Other
//! formats (JPEG, TIFF, ..) are not decoded.

use flate2::read::ZlibDecoder;
use std::io::Read;

/// The largest image decoded, in pixels.
const MAX_PIXELS: usize = 1 << 26;

/// An image as grey values from 0 (black) to 1 (white), row after row from the top.
#[derive(Debug, Clone, PartialEq)]
pub struct GreyImage {
    pub width: usize,
    pub height: usize,
    pub values: Vec<f32>,
}

impl GreyImage {
    /// The grey value at texture coordinates (u to the right, v up, as Rhino maps them),
    /// repeated outside 0..1 and interpolated between pixel centres.
    pub fn sample(&self, u: f64, v: f64) -> f64 {
        let (w, h) = (self.width as f64, self.height as f64);
        let x = (u - u.floor()) * w - 0.5;
        let y = (1.0 - (v - v.floor())) * h - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let at = |i: f64, j: f64| {
            let i = (i.max(0.0) as usize).min(self.width - 1);
            let j = (j.max(0.0) as usize).min(self.height - 1);
            self.values[j * self.width + i] as f64
        };
        let top = at(x0, y0) * (1.0 - fx) + at(x0 + 1.0, y0) * fx;
        let bottom = at(x0, y0 + 1.0) * (1.0 - fx) + at(x0 + 1.0, y0 + 1.0) * fx;
        top * (1.0 - fy) + bottom * fy
    }
}

/// Decodes a PNG or BMP image; `None` for other formats or a damaged image.
pub fn decode(data: &[u8]) -> Option<GreyImage> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        png(data)
    } else if data.starts_with(b"BM") {
        bmp(data)
    } else {
        None
    }
}

fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn grey(r: f32, g: f32, b: f32) -> f32 {
    0.299 * r + 0.587 * g + 0.114 * b
}

fn png(d: &[u8]) -> Option<GreyImage> {
    let (mut at, mut ihdr, mut palette, mut idat) = (8, None, Vec::new(), Vec::new());
    while at + 8 <= d.len() {
        let len = be32(d, at)? as usize;
        let kind = d.get(at + 4..at + 8)?;
        let body = d.get(at + 8..at + 8 + len)?;
        match kind {
            b"IHDR" => ihdr = Some(body.to_vec()),
            b"PLTE" => palette = body.chunks_exact(3).map(|c| grey(c[0] as f32, c[1] as f32, c[2] as f32) / 255.0).collect(),
            b"IDAT" => idat.extend_from_slice(body),
            b"IEND" => break,
            _ => {}
        }
        at += 12 + len;
    }
    let h = ihdr?;
    let (width, height) = (be32(&h, 0)? as usize, be32(&h, 4)? as usize);
    let (depth, color, interlace) = (*h.get(8)?, *h.get(9)?, *h.get(12)?);
    if width == 0 || height == 0 || width.checked_mul(height)? > MAX_PIXELS || !matches!(depth, 1 | 2 | 4 | 8 | 16) {
        return None;
    }
    let channels = match color {
        0 | 3 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => return None,
    };
    if color == 3 && palette.is_empty() {
        return None;
    }
    let bits = channels * depth as usize;
    let bpp = bits.div_ceil(8);
    let mut raw = Vec::new();
    ZlibDecoder::new(&idat[..]).take((MAX_PIXELS * 8 + height * 8) as u64).read_to_end(&mut raw).ok()?;
    let mut values = vec![0f32; width * height];
    let max = ((1u32 << depth) - 1) as f32;
    // the passes of Adam7, or the whole image
    let passes: &[(usize, usize, usize, usize)] = if interlace == 1 {
        &[(0, 0, 8, 8), (4, 0, 8, 8), (0, 4, 4, 8), (2, 0, 4, 4), (0, 2, 2, 4), (1, 0, 2, 2), (0, 1, 1, 2)]
    } else {
        &[(0, 0, 1, 1)]
    };
    let mut pos = 0;
    for &(x0, y0, dx, dy) in passes {
        let (pw, ph) = ((width + dx - 1 - x0) / dx, (height + dy - 1 - y0) / dy);
        if x0 >= width || y0 >= height || pw == 0 || ph == 0 {
            continue;
        }
        let stride = (pw * bits).div_ceil(8);
        let mut prev = vec![0u8; stride];
        for row in 0..ph {
            let filter = *raw.get(pos)?;
            let mut line = raw.get(pos + 1..pos + 1 + stride)?.to_vec();
            pos += 1 + stride;
            unfilter(filter, &mut line, &prev, bpp)?;
            for col in 0..pw {
                let sample = |k: usize| -> f32 {
                    let bit = (col * channels + k) * depth as usize;
                    match depth {
                        16 => u16::from_be_bytes([line[bit / 8], line[bit / 8 + 1]]) as f32,
                        8 => line[bit / 8] as f32,
                        _ => ((line[bit / 8] >> (8 - depth as usize - bit % 8)) & ((1 << depth) - 1)) as f32,
                    }
                };
                let v = match color {
                    0 | 4 => sample(0) / max,
                    3 => *palette.get(sample(0) as usize)?,
                    _ => grey(sample(0), sample(1), sample(2)) / max,
                };
                values[(y0 + row * dy) * width + x0 + col * dx] = v;
            }
            prev = line;
        }
    }
    Some(GreyImage { width, height, values })
}

fn unfilter(filter: u8, line: &mut [u8], prev: &[u8], bpp: usize) -> Option<()> {
    for i in 0..line.len() {
        let a = if i >= bpp { line[i - bpp] as i16 } else { 0 };
        let b = prev[i] as i16;
        let c = if i >= bpp { prev[i - bpp] as i16 } else { 0 };
        let add = match filter {
            0 => 0,
            1 => a,
            2 => b,
            3 => (a + b) / 2,
            4 => {
                let p = a + b - c;
                let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
                if pa <= pb && pa <= pc {
                    a
                } else if pb <= pc {
                    b
                } else {
                    c
                }
            }
            _ => return None,
        };
        line[i] = line[i].wrapping_add(add as u8);
    }
    Some(())
}

fn bmp(d: &[u8]) -> Option<GreyImage> {
    let offset = le32(d, 10)? as usize;
    let width = le32(d, 18)? as i32;
    let height = le32(d, 22)? as i32;
    let bits = u16::from_le_bytes(d.get(28..30)?.try_into().ok()?) as usize;
    let compression = le32(d, 30)?;
    if width <= 0 || height == 0 || compression != 0 && compression != 3 || !matches!(bits, 8 | 24 | 32) {
        return None;
    }
    let (width, flip) = (width as usize, height > 0);
    let height = height.unsigned_abs() as usize;
    if width.checked_mul(height)? > MAX_PIXELS {
        return None;
    }
    let header = le32(d, 14)? as usize;
    let palette: Vec<f32> = if bits == 8 {
        let n = match le32(d, 46)? {
            0 => 256,
            n => n.min(256) as usize,
        };
        d.get(14 + header..14 + header + 4 * n)?.chunks_exact(4).map(|c| grey(c[2] as f32, c[1] as f32, c[0] as f32) / 255.0).collect()
    } else {
        Vec::new()
    };
    let stride = (width * bits).div_ceil(32) * 4;
    let mut values = vec![0f32; width * height];
    for row in 0..height {
        let line = d.get(offset + row * stride..offset + row * stride + width * bits / 8)?;
        let y = if flip { height - 1 - row } else { row };
        for x in 0..width {
            values[y * width + x] = match bits {
                8 => *palette.get(line[x] as usize)?,
                _ => {
                    let p = &line[x * bits / 8..];
                    grey(p[2] as f32, p[1] as f32, p[0] as f32) / 255.0
                }
            };
        }
    }
    Some(GreyImage { width, height, values })
}
