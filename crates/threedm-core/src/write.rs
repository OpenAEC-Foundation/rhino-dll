//! A minimal 3dm writer, for tests: archives as openNURBS writes them, for the parts the
//! readers use. Not a general 3dm writer.

use crate::*;
use glam::DMat4;
use std::io::Write;

/// Writes 3dm archives as openNURBS does, for the parts the reader uses: chunks with 4- or
/// 8-byte lengths (CRCs written as zero, the reader does not check them).
pub struct W {
    big: bool,
}

pub fn i32b(v: i32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

pub fn f64s(v: &[f64]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

pub fn wstr(s: &str) -> Vec<u8> {
    if s.is_empty() {
        return i32b(0);
    }
    let units: Vec<u16> = s.encode_utf16().chain([0]).collect();
    let mut out = i32b(units.len() as i32);
    out.extend(units.iter().flat_map(|u| u.to_le_bytes()));
    out
}

pub fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

impl W {
    pub fn chunk(&self, typecode: u32, body: &[u8]) -> Vec<u8> {
        let crc = if typecode & TCODE_CRC != 0 { 4 } else { 0 };
        let length = body.len() + crc;
        let mut out = typecode.to_le_bytes().to_vec();
        if self.big {
            out.extend((length as i64).to_le_bytes());
        } else {
            out.extend((length as u32).to_le_bytes());
        }
        out.extend(body);
        out.extend(vec![0; crc]);
        out
    }

    pub fn short(&self, typecode: u32, value: i64) -> Vec<u8> {
        let mut out = typecode.to_le_bytes().to_vec();
        if self.big {
            out.extend(value.to_le_bytes());
        } else {
            out.extend((value as i32).to_le_bytes());
        }
        out
    }

    /// `BeginWrite3dmChunk(typecode, major, minor)`.
    pub fn versioned(&self, typecode: u32, major: i32, minor: i32, body: &[u8]) -> Vec<u8> {
        self.chunk(typecode, &cat(&[&i32b(major), &i32b(minor), body]))
    }

    /// `WriteObject`, with user data entries (class id, data).
    pub fn object(&self, class: Uuid, data: &[u8], userdata: &[(Uuid, Vec<u8>)]) -> Vec<u8> {
        let mut body = self.chunk(TCODE_OPENNURBS_CLASS_UUID, &class.0);
        if !class.is_nil() {
            body.extend(self.chunk(TCODE_OPENNURBS_CLASS_DATA, data));
            for (id, ud) in userdata {
                body.extend(self.user_data(*id, ud));
            }
            body.extend(self.short(TCODE_OPENNURBS_CLASS_END, 0));
        }
        self.chunk(TCODE_OPENNURBS_CLASS, &body)
    }

    pub fn user_data(&self, class: Uuid, data: &[u8]) -> Vec<u8> {
        let header = self.chunk(TCODE_OPENNURBS_CLASS_USERDATA_HEADER, &cat(&[&class.0, &[0; 16], &i32b(1), &f64s(&DMat4::IDENTITY.to_cols_array())]));
        self.chunk(TCODE_OPENNURBS_CLASS_USERDATA, &cat(&[&[0x20], &header, &self.chunk(TCODE_ANONYMOUS_CHUNK, data)]))
    }

    pub fn user_strings(&self, pairs: &[(&str, &str)]) -> Vec<u8> {
        let mut list = i32b(pairs.len() as i32);
        for (k, v) in pairs {
            list.extend(self.versioned(TCODE_ANONYMOUS_CHUNK, 1, 0, &cat(&[&wstr(k), &wstr(v)])));
        }
        self.versioned(TCODE_ANONYMOUS_CHUNK, 1, 0, &list)
    }

    /// `WriteCompressedBuffer`, stored (method 0) or deflated (method 1).
    pub fn compressed(&self, bytes: &[u8], deflate: bool) -> Vec<u8> {
        if bytes.is_empty() {
            return i32b(0);
        }
        let mut out = cat(&[&(bytes.len() as u32).to_le_bytes(), &[0; 4]]);
        if deflate {
            let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            z.write_all(bytes).unwrap();
            out.push(1);
            out.extend(self.chunk(TCODE_ANONYMOUS_CHUNK, &z.finish().unwrap()));
        } else {
            out.push(0);
            out.extend(bytes);
        }
        out
    }

    /// An `ON_Mesh` 3.x: points, faces (a triangle repeats its last index), optional
    /// normals and colours.
    pub fn mesh(&self, points: &[[f32; 3]], faces: &[[i32; 4]], normals: Option<&[[f32; 3]]>, colors: Option<&[[u8; 4]]>, deflate: bool) -> Vec<u8> {
        let floats = |v: &[[f32; 3]]| -> Vec<u8> { v.iter().flatten().flat_map(|f| f.to_le_bytes()).collect() };
        let mut out = vec![0x30];
        out.extend(i32b(points.len() as i32));
        out.extend(i32b(faces.len() as i32));
        out.extend(vec![0; 148]);
        out.extend([0; 5]);
        out.extend(i32b(4));
        out.extend(faces.iter().flatten().flat_map(|i| i.to_le_bytes()));
        out.extend(self.compressed(&floats(points), deflate));
        out.extend(self.compressed(&normals.map(floats).unwrap_or_default(), deflate));
        out.extend(i32b(0));
        out.extend(i32b(0));
        out.extend(self.compressed(&colors.map(|c| c.concat()).unwrap_or_default(), deflate));
        out
    }

    /// An `ON_Brep` 3.2 whose faces carry the given render meshes; the first face is red.
    pub fn brep(&self, meshes: &[Vec<u8>]) -> Vec<u8> {
        let mut out = vec![0x32];
        for _ in 0..7 {
            out.extend(self.chunk(TCODE_ANONYMOUS_CHUNK, &cat(&[&[0x10], &i32b(0)])));
        }
        // the faces (index, loops, surface, reversed, material channel), their ids, and a colour
        // for the first face only
        let mut faces = cat(&[&[0x12], &i32b(meshes.len() as i32)]);
        for k in 0..meshes.len() {
            faces.extend(cat(&[&i32b(k as i32), &i32b(1), &i32b(0), &i32b(k as i32), &i32b(0), &i32b(0)]));
        }
        faces.extend(vec![7; 16 * meshes.len()]);
        faces.push(1);
        for k in 0..meshes.len() {
            faces.extend(if k == 0 { [255, 0, 0, 0] } else { [255; 4] });
        }
        out.extend(self.chunk(TCODE_ANONYMOUS_CHUNK, &faces));
        out.extend(f64s(&[0.0; 6]));
        let mut render = Vec::new();
        for m in meshes {
            render.push(1);
            render.extend(self.object(ON_MESH, m, &[]));
        }
        out.extend(self.chunk(TCODE_ANONYMOUS_CHUNK, &render));
        out.extend(self.chunk(TCODE_ANONYMOUS_CHUNK, &vec![0; meshes.len()]));
        out.extend(i32b(2));
        out
    }

    #[allow(clippy::too_many_arguments)]
    pub fn layer(&self, index: i32, name: &str, color: [u8; 4], visible: bool, id: Uuid, parent: Uuid, material: i32) -> Vec<u8> {
        let data = cat(&[
            &[0x16],
            &i32b(if visible { 0 } else { 1 }),
            &i32b(index),
            &i32b(0),
            &i32b(material),
            &i32b(0),
            &color,
            &[0; 20],
            &wstr(name),
            &[visible as u8],
            &i32b(-1),
            &[0; 12],
            &[0],
            &id.0,
            &parent.0,
            &[1],
        ]);
        self.chunk(TCODE_LAYER_RECORD, &self.object(ON_LAYER, &data, &[]))
    }

    pub fn table(&self, typecode: u32, records: &[Vec<u8>]) -> Vec<u8> {
        let mut body = records.concat();
        body.extend(self.short(TCODE_ENDOFTABLE, 0));
        self.chunk(typecode, &body)
    }

    pub fn record(&self, object_type: u32, object: &[u8], attributes: &[u8], user: &[(&str, &str)]) -> Vec<u8> {
        let mut body = self.short(TCODE_OBJECT_RECORD_TYPE, object_type as i64);
        body.extend(object);
        body.extend(self.chunk(TCODE_OBJECT_RECORD_ATTRIBUTES, attributes));
        if !user.is_empty() {
            let mut ud = self.user_data(ON_USER_STRING_LIST, &self.user_strings(user));
            ud.extend(self.short(TCODE_OPENNURBS_CLASS_END, 0));
            body.extend(self.chunk(TCODE_OBJECT_RECORD_ATTRIBUTES_USERDATA, &ud));
        }
        body.extend(self.short(TCODE_OBJECT_RECORD_END, 0));
        self.chunk(TCODE_OBJECT_RECORD, &body)
    }

    /// A file: header, comment, properties, units settings, then the tables.
    pub fn file(&self, archive: u32, opennurbs: u32, unit: i32, tables: &[Vec<u8>]) -> Vec<u8> {
        let mut out = format!("3D Geometry File Format {archive:>8}").into_bytes();
        out.extend(self.chunk(1, b"written by the threedm tests"));
        let application = self.chunk(TCODE_PROPERTIES_APPLICATION, &cat(&[&[0x10], &wstr("Rhinoceros 7"), &wstr(""), &wstr("")]));
        out.extend(self.table(TCODE_PROPERTIES_TABLE, &[self.short(TCODE_PROPERTIES_OPENNURBS_VERSION, opennurbs as i64), application]));
        let units = cat(&[&i32b(102), &i32b(unit), &f64s(&[0.001, 1.0, 0.01]), &i32b(0), &i32b(3), &f64s(&[1.0]), &wstr("")]);
        out.extend(self.table(TCODE_SETTINGS_TABLE, &[self.chunk(TCODE_SETTINGS_UNITSANDTOLS, &units)]));
        for t in tables {
            out.extend(t);
        }
        let size = out.len() as u64 + if self.big { 16 } else { 12 };
        out.extend(if self.big { self.chunk(TCODE_ENDOFFILE, &size.to_le_bytes()) } else { self.chunk(TCODE_ENDOFFILE, &(size as u32).to_le_bytes()) });
        out
    }
}

/// Rhino 5+ object attributes: id, layer and the given items.
pub fn attributes(id: Uuid, layer: i32, items: &[(u8, Vec<u8>)]) -> Vec<u8> {
    let mut out = cat(&[&[0x20], &id.0, &i32b(layer)]);
    for (item, value) in items {
        out.push(*item);
        out.extend(value);
    }
    out.push(0);
    out
}

/// A uuid of sixteen equal bytes.
pub fn id(n: u8) -> Uuid {
    Uuid([n; 16])
}
