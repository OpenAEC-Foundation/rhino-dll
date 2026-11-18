//! Container layer of the 3dm reader: the openNURBS archive. Little-endian values in nested
//! chunks (4- and 8-byte chunk lengths), versioned and anonymous chunks, compressed buffers,
//! `ReadObject` (class id, class data and user data) and the user strings ("user text") of
//! objects. Written from McNeel's openNURBS source, the published reference of the format;
//! nothing of openNURBS is compiled in.

use glam::DMat4;
use std::io::Read;

pub mod write;

pub type R<T> = Result<T, String>;

pub fn err<T>(what: &str) -> R<T> {
    Err(format!("Rhino 3dm: {what}"))
}

// ---------------------------------------------------------------------------------------------
// Typecodes (openNURBS `opennurbs_3dm.h`).

pub const TCODE_SHORT: u32 = 0x8000_0000;
pub const TCODE_CRC: u32 = 0x8000;
pub const TCODE_ENDOFFILE: u32 = 0x0000_7FFF;
pub const TCODE_ENDOFTABLE: u32 = 0xFFFF_FFFF;
pub const TCODE_ANONYMOUS_CHUNK: u32 = 0x4000_8000;
pub const TCODE_MODEL_ATTRIBUTES_CHUNK: u32 = 0x4000_8002;
pub const TCODE_MATERIAL_TABLE: u32 = 0x1000_0010;
pub const TCODE_LAYER_TABLE: u32 = 0x1000_0011;
pub const TCODE_OBJECT_TABLE: u32 = 0x1000_0013;
pub const TCODE_PROPERTIES_TABLE: u32 = 0x1000_0014;
pub const TCODE_SETTINGS_TABLE: u32 = 0x1000_0015;
pub const TCODE_INSTANCE_DEFINITION_TABLE: u32 = 0x1000_0021;
pub const TCODE_HATCHPATTERN_TABLE: u32 = 0x1000_0022;
pub const TCODE_DIMSTYLE_TABLE: u32 = 0x1000_0020;
pub const TCODE_DIMSTYLE_RECORD: u32 = 0x2000_8075;
pub const TCODE_HATCHPATTERN_RECORD: u32 = 0x2000_8077;
pub const TCODE_PROPERTIES_APPLICATION: u32 = 0x2000_8024;
pub const TCODE_PROPERTIES_OPENNURBS_VERSION: u32 = 0xA000_0026;
pub const TCODE_SETTINGS_UNITSANDTOLS: u32 = 0x2000_8031;
pub const TCODE_MATERIAL_RECORD: u32 = 0x2000_8040;
pub const TCODE_LAYER_RECORD: u32 = 0x2000_8050;
pub const TCODE_OBJECT_RECORD: u32 = 0x2000_8070;
pub const TCODE_INSTANCE_DEFINITION_RECORD: u32 = 0x2000_8076;
pub const TCODE_OBJECT_RECORD_TYPE: u32 = 0x8200_0071;
pub const TCODE_OBJECT_RECORD_ATTRIBUTES: u32 = 0x0200_8072;
pub const TCODE_OBJECT_RECORD_ATTRIBUTES_USERDATA: u32 = 0x0200_0073;
pub const TCODE_OBJECT_RECORD_END: u32 = 0x8200_007F;
pub const TCODE_OPENNURBS_CLASS: u32 = 0x0002_7FFA;
pub const TCODE_OPENNURBS_CLASS_UUID: u32 = 0x0002_FFFB;
pub const TCODE_OPENNURBS_CLASS_DATA: u32 = 0x0002_FFFC;
pub const TCODE_OPENNURBS_CLASS_USERDATA: u32 = 0x0002_7FFD;
pub const TCODE_OPENNURBS_CLASS_USERDATA_HEADER: u32 = 0x0002_FFF9;
pub const TCODE_OPENNURBS_CLASS_END: u32 = 0x8002_7FFF;

/// A UUID as stored: `Data1` (u32), `Data2`, `Data3` (u16, little endian) and `Data4` (8 bytes).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct Uuid(pub [u8; 16]);

impl Uuid {
    /// From the usual text form, `4ED7D4E4-E947-11d3-BFE5-0010830122F0`.
    pub const fn parse(s: &str) -> Uuid {
        let b = s.as_bytes();
        let mut hex = [0u8; 16];
        let (mut i, mut n) = (0, 0);
        while i < b.len() {
            let c = b[i];
            i += 1;
            let v = match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'f' => c - b'a' + 10,
                b'A'..=b'F' => c - b'A' + 10,
                _ => continue,
            };
            hex[n / 2] |= if n % 2 == 0 { v << 4 } else { v };
            n += 1;
        }
        Uuid([hex[3], hex[2], hex[1], hex[0], hex[5], hex[4], hex[7], hex[6], hex[8], hex[9], hex[10], hex[11], hex[12], hex[13], hex[14], hex[15]])
    }

    pub fn is_nil(&self) -> bool {
        self.0 == [0; 16]
    }
}

impl std::fmt::Display for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let b = &self.0;
        write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            b[3], b[2], b[1], b[0], b[5], b[4], b[7], b[6], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
        )
    }
}

impl std::fmt::Debug for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

pub const ON_MESH: Uuid = Uuid::parse("4ED7D4E4-E947-11d3-BFE5-0010830122F0");
pub const ON_BREP: Uuid = Uuid::parse("60B5DBC5-E660-11d3-BFE4-0010830122F0");
/// The class ids Rhino writes Breps with (`TL_Brep`) and those of older files, read as `ON_Brep`.
pub const BREP_ALIASES: [Uuid; 3] = [
    Uuid::parse("F06FC243-A32A-4608-9DD8-A7D2C4CE2A36"),
    Uuid::parse("0705FDEF-3E2A-11d4-800E-0010830122F0"),
    Uuid::parse("2D4CFEDB-3E2A-11d4-800E-0010830122F0"),
];
pub const ON_EXTRUSION: Uuid = Uuid::parse("36F53175-72B8-4d47-BF1F-B4E6FC24F4B9");
pub const ON_SUBD: Uuid = Uuid::parse("F09BA4D9-455B-42C3-BA3B-E6CCACEF853B");
pub const ON_POINT_CLOUD: Uuid = Uuid::parse("2488F347-F8FA-11d3-BFEC-0010830122F0");
pub const ON_POINT: Uuid = Uuid::parse("C3101A1D-F157-11d3-BFE7-0010830122F0");
pub const ON_INSTANCE_REF: Uuid = Uuid::parse("F9CFB638-B9D4-4340-87E3-C56E7865D96A");
pub const ON_INSTANCE_DEFINITION: Uuid = Uuid::parse("26F8BFF6-2618-417f-A158-153D64A94989");
pub const ON_LAYER: Uuid = Uuid::parse("95809813-E985-11d3-BFE5-0010830122F0");
pub const ON_MATERIAL: Uuid = Uuid::parse("60B5DBBC-E660-11d3-BFE4-0010830122F0");
pub const ON_USER_STRING_LIST: Uuid = Uuid::parse("CE28DE29-F4C5-4faa-A50A-C3A6849B6329");
/// The render and analysis meshes of a Rhino 5 extrusion, kept as user data.
pub const ON_V5_EXTRUSION_MESH_CACHE: Uuid = Uuid::parse("A8130A3E-E4F3-4CB0-BB8A-F10A473912D0");
/// `ON_MeshCache::RenderMeshId`.
pub const RENDER_MESH_ID: Uuid = Uuid::parse("F5E3BAA9-A7A2-49FD-B8A1-66EB274A5F91");

// ---------------------------------------------------------------------------------------------
// The archive: little-endian values in nested chunks. A chunk is a typecode and a value; a
// "short" chunk (high bit of the typecode) has only the value, a long one is followed by value
// bytes of content, the last 4 a CRC when the typecode has the CRC bit. Lengths are 4 bytes
// before archive version 50 and 8 bytes from it.

#[derive(Clone, Copy, Debug, Default)]
pub struct Versions {
    /// 2, 3, 4, 5, 50, 60, 70, 80.
    pub archive: u32,
    /// The openNURBS version that wrote the file (`YYYYMMDDn` up to Rhino 5, larger after).
    pub opennurbs: u32,
}

#[derive(Clone)]
pub struct Cur<'a> {
    pub data: &'a [u8],
    pub pos: usize,
    pub v: Versions,
}

pub struct Chunk<'a> {
    pub typecode: u32,
    pub value: i64,
    pub body: Cur<'a>,
}

impl<'a> Cur<'a> {
    pub fn left(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn take(&mut self, n: usize) -> R<&'a [u8]> {
        if n > self.left() {
            return err("unexpected end of a record");
        }
        let out = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    pub fn array<const N: usize>(&mut self) -> R<[u8; N]> {
        Ok(self.take(N)?.try_into().unwrap())
    }

    pub fn u8(&mut self) -> R<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn bool(&mut self) -> R<bool> {
        Ok(self.u8()? != 0)
    }

    pub fn i32(&mut self) -> R<i32> {
        Ok(i32::from_le_bytes(self.array()?))
    }

    pub fn u32(&mut self) -> R<u32> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub fn i64(&mut self) -> R<i64> {
        Ok(i64::from_le_bytes(self.array()?))
    }

    pub fn f64(&mut self) -> R<f64> {
        Ok(f64::from_le_bytes(self.array()?))
    }

    pub fn uuid(&mut self) -> R<Uuid> {
        Ok(Uuid(self.array()?))
    }

    /// `ON_Color`: red, green, blue and alpha bytes, alpha 0 opaque.
    pub fn color(&mut self) -> R<[u8; 4]> {
        self.array()
    }

    /// `ON_Xform`: 16 doubles, row by row, for column vectors.
    pub fn xform(&mut self) -> R<DMat4> {
        let mut m = [0.0; 16];
        for v in &mut m {
            *v = self.f64()?;
        }
        Ok(DMat4::from_cols_array(&m).transpose())
    }

    /// An element count, at most what the rest of the record can hold at `size` bytes each.
    pub fn count(&mut self, size: usize) -> R<usize> {
        let n = self.i32()?.max(0) as usize;
        if n.saturating_mul(size.max(1)) > self.left() {
            return err("a count larger than its record");
        }
        Ok(n)
    }

    /// `ON_wString`: the number of UTF-16 units (with the terminating zero), then the units.
    pub fn wstring(&mut self) -> R<String> {
        let n = self.u32()? as usize;
        if n == 0 {
            return Ok(String::new());
        }
        if n > self.left() / 2 {
            return err("a string longer than its record");
        }
        let units: Vec<u16> = self.take(2 * n)?.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
        Ok(String::from_utf16_lossy(&units[..end]))
    }

    /// `Read3dmChunkVersion`: major and minor version in one byte.
    pub fn version(&mut self) -> R<(u8, u8)> {
        let v = self.u8()?;
        Ok((v >> 4, v & 15))
    }

    pub fn chunk(&mut self) -> R<Chunk<'a>> {
        let typecode = self.u32()?;
        let short = typecode & TCODE_SHORT != 0;
        let value = if self.v.archive >= 50 {
            self.i64()?
        } else if !short || typecode == TCODE_PROPERTIES_OPENNURBS_VERSION || typecode == TCODE_OBJECT_RECORD_TYPE {
            self.u32()? as i64
        } else {
            self.i32()? as i64
        };
        if short {
            return Ok(Chunk { typecode, value, body: Cur { data: &[], pos: 0, v: self.v } });
        }
        let length = usize::try_from(value).ok().filter(|&l| l <= self.left()).ok_or_else(|| format!("Rhino 3dm: a chunk (typecode {typecode:#x}) runs past its parent"))?;
        let bytes = self.take(length)?;
        let crc = if typecode & TCODE_CRC != 0 && length >= 4 { 4 } else { 0 };
        Ok(Chunk { typecode, value, body: Cur { data: &bytes[..length - crc], pos: 0, v: self.v } })
    }

    /// A chunk of the given typecode that starts with a major and minor version (two ints):
    /// `BeginRead3dmChunk(typecode, &major, &minor)`.
    pub fn versioned(&mut self, typecode: u32) -> R<(Cur<'a>, i32, i32)> {
        let chunk = self.chunk()?;
        if chunk.typecode != typecode {
            return err(&format!("expected chunk {typecode:#x}, found {:#x}", chunk.typecode));
        }
        let mut body = chunk.body;
        let major = body.i32()?;
        let minor = body.i32()?;
        Ok((body, major, minor))
    }

    /// The typecode of the next chunk, without reading it.
    pub fn peek_typecode(&self) -> Option<u32> {
        self.data.get(self.pos..self.pos + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()))
    }

    /// `ReadCompressedBuffer`: the size, then a CRC, a method byte and the bytes, either stored
    /// (method 0) or zlib-deflated in an anonymous chunk (method 1). `None` when empty, and when
    /// the size is not `expected` (the bytes are then skipped without inflating them, so a small
    /// stream cannot claim gigabytes).
    pub fn compressed(&mut self, expected: usize) -> R<Option<Vec<u8>>> {
        let size = self.u32()? as usize;
        if size == 0 {
            return Ok(None);
        }
        let _crc = self.u32()?;
        let method = self.u8()?;
        let packed = match method {
            0 => self.take(size)?,
            1 => {
                let chunk = self.chunk()?;
                &chunk.body.data[chunk.body.pos..]
            }
            _ => return err("unknown compression method"),
        };
        if size != expected {
            return Ok(None);
        }
        if method == 0 {
            return Ok(Some(packed.to_vec()));
        }
        // deflate packs at most 1032 bytes into one: a larger size is a damaged record
        if size > packed.len().saturating_mul(1032).saturating_add(64) {
            return err("compressed data larger than its stream can hold");
        }
        let mut out = Vec::with_capacity(size.min(packed.len().saturating_mul(4)));
        flate2::read::ZlibDecoder::new(packed).take(size as u64).read_to_end(&mut out).map_err(|e| format!("Rhino 3dm: compressed mesh data: {e}"))?;
        if out.len() != size {
            return err("compressed data shorter than its size");
        }
        Ok(Some(out))
    }

    /// `ReadObject`: an openNURBS object of any class, with its user data; `None` for a null
    /// object.
    pub fn object(&mut self) -> R<Option<Object<'a>>> {
        let chunk = self.chunk()?;
        if chunk.typecode != TCODE_OPENNURBS_CLASS {
            return err("expected an object");
        }
        let mut body = chunk.body;
        let id = body.chunk()?;
        if id.typecode != TCODE_OPENNURBS_CLASS_UUID {
            return err("an object without its class id");
        }
        let class = id.body.clone().uuid()?;
        if class.is_nil() {
            return Ok(None);
        }
        let data = body.chunk()?;
        if data.typecode != TCODE_OPENNURBS_CLASS_DATA {
            return err("an object without its data");
        }
        let entries = user_data_entries(&mut body);
        let userdata = entries.iter().map(|e| (e.class, e.data.clone())).collect();
        Ok(Some(Object { class, data: data.body, userdata, entries }))
    }
}

/// An object as read by `ReadObject`: its class id, its data and its user data by class id
/// (`userdata`), and the same user data with their headers (`entries`).
pub struct Object<'a> {
    pub class: Uuid,
    pub data: Cur<'a>,
    pub userdata: Vec<(Uuid, Cur<'a>)>,
    pub entries: Vec<UserData<'a>>,
}

/// A user data entry (`ON_UserData` as `WriteObjectUserData` writes it): the header and the
/// data, as stored.
#[derive(Clone)]
pub struct UserData<'a> {
    /// The class of the user data (`ON_UserData` subclass, or the plug-in's own).
    pub class: Uuid,
    /// `m_userdata_uuid`: which item of the plug-in's data this is.
    pub item: Uuid,
    /// `m_application_uuid`: the plug-in that wrote it (nil before header 2.1).
    pub plugin: Uuid,
    /// The version of the entry's chunk (1.0, 2.0, 2.1 or 2.2).
    pub version: (u8, u8),
    pub copy_count: i32,
    /// `m_userdata_xform`: the transforms applied to the object since the data was attached.
    pub xform: DMat4,
    /// Saved by a program that did not know the class (`ON_UnknownUserData`, "goo").
    pub goo: bool,
    /// The archive and openNURBS versions the data was written with (0 when not stored).
    pub archive: i32,
    pub opennurbs: u32,
    /// The data: the content of its anonymous chunk, read with the archive version it was
    /// written with.
    pub data: Cur<'a>,
}

/// User data entries up to `TCODE_OPENNURBS_CLASS_END` (`ReadObjectUserData`): each a version,
/// a header with the user data class id, and the data in an anonymous chunk. Damaged entries
/// are left out.
pub fn user_data<'a>(c: &mut Cur<'a>) -> Vec<(Uuid, Cur<'a>)> {
    user_data_entries(c).into_iter().map(|e| (e.class, e.data)).collect()
}

/// User data entries with their headers, up to `TCODE_OPENNURBS_CLASS_END`.
pub fn user_data_entries<'a>(c: &mut Cur<'a>) -> Vec<UserData<'a>> {
    let mut out = Vec::new();
    while c.left() > 0 {
        let Ok(chunk) = c.chunk() else { break };
        match chunk.typecode {
            TCODE_OPENNURBS_CLASS_END => break,
            TCODE_OPENNURBS_CLASS_USERDATA => {
                if let Ok(Some(entry)) = user_data_entry(chunk.body) {
                    out.push(entry);
                }
            }
            _ => {}
        }
    }
    out
}

/// One `TCODE_OPENNURBS_CLASS_USERDATA` chunk: its version, its header (in a chunk of its own
/// from 2.0; the plug-in id from 2.1; goo and versions from 2.2) and its data.
pub fn user_data_entry(mut c: Cur<'_>) -> R<Option<UserData<'_>>> {
    let version = c.version()?;
    let mut h = match version.0 {
        2 => {
            let header = c.chunk()?;
            if header.typecode != TCODE_OPENNURBS_CLASS_USERDATA_HEADER {
                return Ok(None);
            }
            header.body
        }
        1 => c.clone(),
        _ => return Ok(None),
    };
    let class = h.uuid()?;
    let item = h.uuid()?;
    let copy_count = h.i32()?;
    let xform = h.xform()?;
    let (mut plugin, mut goo, mut archive, mut opennurbs) = (Uuid::default(), false, 0, 0);
    if version.0 == 2 && version.1 >= 1 {
        plugin = h.uuid()?;
        if version.1 >= 2 {
            goo = h.bool()?;
            archive = h.i32()?;
            opennurbs = h.u32()?;
        }
    }
    if version.0 == 1 {
        c = h;
    }
    let chunk = c.chunk()?;
    if chunk.typecode != TCODE_ANONYMOUS_CHUNK {
        return Ok(None);
    }
    let mut data = chunk.body;
    // the data has the chunk lengths of the archive it was written in: without versions in its
    // header, of Rhino 4 or older (4-byte lengths) when the file is newer
    data.v.archive = match archive {
        0 if c.v.archive >= 50 => 5,
        0 => c.v.archive,
        a => a as u32,
    };
    Ok(Some(UserData { class, item, plugin, version, copy_count, xform, goo, archive, opennurbs, data }))
}

/// `ON_UserStringList`: key and value pairs ("user text").
pub fn user_strings(mut c: Cur<'_>) -> R<Vec<(String, String)>> {
    let (mut list, major, _) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    if major != 1 {
        return Ok(Vec::new());
    }
    // every entry is a chunk of at least 16 bytes: typecode, length, major and minor version
    let count = list.count(16)?;
    let mut out = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let (mut entry, major, _) = list.versioned(TCODE_ANONYMOUS_CHUNK)?;
        if major == 1 {
            let key = entry.wstring()?;
            let value = entry.wstring()?;
            if !key.is_empty() {
                out.push((key, value));
            }
        }
    }
    Ok(out)
}

pub fn user_strings_of(userdata: &[(Uuid, Cur<'_>)]) -> Vec<(String, String)> {
    userdata.iter().filter(|(class, _)| *class == ON_USER_STRING_LIST).flat_map(|(_, c)| user_strings(c.clone()).unwrap_or_default()).collect()
}
