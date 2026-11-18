//! Plug-in data: the user data of objects and table records (`ON_UserData`, every entry kept
//! as stored with its header), the document's user tables (`TCODE_USER_TABLE`), the document
//! user text (`ON_DocumentUserStringList`) and the file's plug-in list
//! (read with the settings) that names the plug-ins. The user tables themselves are read by
//! `settings`; this decodes what of them is generic. What has a generic, self-describing
//! form is decoded: user text (`ON_UserStringList`), archivable dictionaries (RhinoCommon's
//! `UserDictionary`, `ON_ArchivableDictionary`) and XML or JSON text.

use threedm_core::*;
use threedm_model::{Decoded, Dictionary, Entry, Owner, PlugInData, UserTable, Value};

/// The archivable dictionary chunks (`BeginWriteDictionary`).
const TCODE_DICTIONARY: u32 = 0x4000_8010;
const TCODE_DICTIONARY_ID: u32 = 0x4000_8011;
const TCODE_DICTIONARY_ENTRY: u32 = 0x4000_8012;
const TCODE_DICTIONARY_END: u32 = 0xC000_0013;

/// `ON_DocumentUserStringList`: the user table (by this plug-in id) of the document's user text.
pub(crate) const ON_DOCUMENT_USER_STRING_LIST: Uuid = Uuid::parse("06F3218E-F5EC-4f6c-B74C-14583F0ED7BC");

const MAX_DEPTH: usize = 16;

/// Decodes a user table read by `settings::read_user_table` where its form is generic: the
/// document user text (`ON_DocumentUserStringList`), a dictionary, XML or JSON text. Its data
/// has the chunk lengths of the archive it was written in (Rhino 4's, when the file does not
/// say and is newer).
pub(crate) fn decode_user_table(t: &mut UserTable<'_>, file: Versions) {
    let archive = match t.archive_version {
        0 if file.archive >= 50 => 5,
        0 => file.archive,
        a => a as u32,
    };
    let data = Cur { data: t.data, pos: 0, v: Versions { archive, opennurbs: file.opennurbs } };
    t.decoded = if t.plugin == ON_DOCUMENT_USER_STRING_LIST {
        // `WriteObject` of an `ON_DocumentUserStringList`: its strings are its user data
        data.clone().object().ok().flatten().map(|o| Decoded::UserText(user_strings_of(&o.userdata)))
    } else {
        decode(Uuid::default(), data)
    };
}

/// Of an object record (`index` in the file's objects): the user data of its geometry and of
/// its attributes as plug-in data, and the parameters of the rarer classes ([`crate::rare`]).
pub(crate) fn object_record<'a>(mut c: Cur<'a>, index: usize, file: &mut threedm_model::File<'a>) {
    let Ok(kind) = c.chunk() else { return };
    if kind.typecode != TCODE_OBJECT_RECORD_TYPE {
        return;
    }
    if let Ok(Some(object)) = c.object() {
        file.plugin_data.extend(records(Owner::Object { index, attributes: false }, &object.entries));
        crate::rare::extras(object.class, object.data, index, file);
    }
    while c.left() > 0 {
        let Ok(chunk) = c.chunk() else { break };
        match chunk.typecode {
            TCODE_OBJECT_RECORD_ATTRIBUTES_USERDATA => {
                let mut body = chunk.body;
                file.plugin_data.extend(records(Owner::Object { index, attributes: true }, &user_data_entries(&mut body)));
            }
            TCODE_OBJECT_RECORD_END => break,
            _ => {}
        }
    }
}

/// The user data of a table record that is an object (a layer, material, ..), as plug-in data.
pub(crate) fn table_record<'a>(record: Cur<'a>, table: &'static str, index: usize, file: &mut threedm_model::File<'a>) {
    if let Ok(Some(object)) = record.clone().object() {
        file.plugin_data.extend(records(Owner::Table { table, index }, &object.entries));
    }
}

/// The user data entries of an owner as plug-in data.
pub(crate) fn records<'a>(owner: Owner, entries: &[UserData<'a>]) -> Vec<PlugInData<'a>> {
    entries.iter().map(|e| PlugInData {
        owner,
        class: e.class,
        plugin: e.plugin,
        plugin_name: String::new(),
        item: e.item,
        version: e.version,
        copy_count: e.copy_count,
        xform: e.xform,
        goo: e.goo,
        archive: e.archive,
        opennurbs: e.opennurbs,
        bytes: &e.data.data[e.data.pos..],
        decoded: decode(e.class, e.data.clone()),
    }).collect()
}

/// Names the plug-ins of the plug-in data from the file's plug-in list.
pub(crate) fn name_plugins(file: &mut threedm_model::File<'_>) {
    let plugins = &file.settings.plugins;
    for d in &mut file.plugin_data {
        if let Some(p) = plugins.iter().find(|p| p.id == d.plugin) {
            d.plugin_name = p.name.clone();
        }
    }
}

/// What the data holds, when its form is known: user text, a dictionary, or XML or JSON text.
pub(crate) fn decode(class: Uuid, data: Cur<'_>) -> Option<Decoded> {
    if class == ON_USER_STRING_LIST {
        return user_strings(data).ok().map(Decoded::UserText);
    }
    if data.peek_typecode() == Some(TCODE_DICTIONARY) {
        return dictionary(&mut data.clone(), 0).ok().map(Decoded::Dictionary);
    }
    text(&data.data[data.pos..]).map(Decoded::Text)
}

/// XML or JSON text at the start of the data (within its first 64 bytes, after the versions and
/// lengths that come before it): UTF-8 with a length before it or up to the end, or UTF-16 with
/// its count of units before it.
fn text(bytes: &[u8]) -> Option<String> {
    let starts = |b: u8| matches!(b, b'<' | b'{' | b'[');
    let printable = |s: &str| s.chars().all(|c| !c.is_control() || c.is_whitespace());
    for k in 0..bytes.len().min(64) {
        if !starts(bytes[k]) {
            continue;
        }
        // UTF-16, its unit count (with the zero) before it
        if k >= 4 && bytes.get(k + 1) == Some(&0) {
            let n = u32::from_le_bytes(bytes[k - 4..k].try_into().unwrap()) as usize;
            if n >= 2 && k + 2 * n <= bytes.len() {
                let units: Vec<u16> = bytes[k..k + 2 * n].as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
                let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
                if let Ok(s) = String::from_utf16(&units[..end]) {
                    if printable(&s) {
                        return Some(s);
                    }
                }
            }
        }
        // UTF-8: its byte count before it, or to the end
        let counted = (k >= 4).then(|| u32::from_le_bytes(bytes[k - 4..k].try_into().unwrap()) as usize).filter(|&n| n >= 2 && k + n <= bytes.len());
        let end = counted.map_or(bytes.len(), |n| k + n);
        let raw = &bytes[k..end];
        let raw = &raw[..raw.iter().position(|&b| b == 0).unwrap_or(raw.len())];
        if let Ok(s) = std::str::from_utf8(raw) {
            let t = s.trim_end();
            if t.len() >= 2 && printable(t) && matches!(t.as_bytes()[t.len() - 1], b'>' | b'}' | b']') {
                return Some(t.to_string());
            }
        }
        return None;
    }
    None
}

/// An archivable dictionary (`ON_BinaryArchive::BeginReadDictionary`): the dictionary chunk
/// (version 1), its id chunk (id, version, name), then its entries up to the end chunk, each in
/// a chunk of its own: item type, key, value.
pub(crate) fn dictionary(c: &mut Cur<'_>, depth: usize) -> R<Dictionary> {
    if depth > MAX_DEPTH {
        return err("dictionaries nested too deep");
    }
    let (mut b, major, _) = c.versioned(TCODE_DICTIONARY)?;
    if major != 1 {
        return err("a dictionary of an unknown version");
    }
    let (mut id, major, _) = b.versioned(TCODE_DICTIONARY_ID)?;
    if major != 1 {
        return err("a dictionary of an unknown version");
    }
    let mut d = Dictionary { id: id.uuid()?, version: id.u32()?, name: id.wstring()?, entries: Vec::new() };
    while b.left() > 0 {
        let chunk = b.chunk()?;
        match chunk.typecode {
            TCODE_DICTIONARY_END => break,
            TCODE_DICTIONARY_ENTRY => {
                let mut e = chunk.body;
                let kind = e.i32()?;
                let key = e.wstring()?;
                let start = e.clone();
                let value = match entry_value(kind, &mut e, depth) {
                    Ok(v) if e.left() == 0 => v,
                    // a type not known, or not read to its end: as stored
                    _ => Value::Raw(start.data[start.pos..].to_vec()),
                };
                d.entries.push(Entry { key, kind, value });
            }
            _ => {}
        }
    }
    Ok(d)
}

/// The name of a dictionary item type (`DictionaryEntryType`).
pub fn entry_type(kind: i32) -> &'static str {
    const NAMES: [&str; 48] = [
        "undefined", "bool", "uint8", "int8", "int16", "uint16", "int32", "uint32", "int64", "float", "double", "uuid", "string",
        "bool[]", "uint8[]", "int8[]", "int16[]", "int32[]", "float[]", "double[]", "uuid[]", "string[]",
        "color", "point2i", "point2f", "rect4i", "rect4f", "size2i", "size2f", "font", "interval", "point2d", "point3d", "point4d",
        "vector2d", "vector3d", "bounding box", "ray", "plane equation", "xform", "plane", "line", "point3f", "vector3f",
        "dictionary", "object", "mesh parameters", "geometry",
    ];
    usize::try_from(kind).ok().and_then(|k| NAMES.get(k)).copied().unwrap_or("unknown")
}

fn doubles(c: &mut Cur<'_>, n: usize) -> R<Value> {
    Ok(Value::Numbers((0..n).map(|_| c.f64()).collect::<R<_>>()?))
}

fn floats(c: &mut Cur<'_>, n: usize) -> R<Value> {
    Ok(Value::Numbers((0..n).map(|_| c.array::<4>().map(|b| f32::from_le_bytes(b) as f64)).collect::<R<_>>()?))
}

fn ints(c: &mut Cur<'_>, n: usize) -> R<Value> {
    Ok(Value::Numbers((0..n).map(|_| c.i32().map(f64::from)).collect::<R<_>>()?))
}

/// An array: a count, then as many items.
fn array(c: &mut Cur<'_>, size: usize, mut item: impl FnMut(&mut Cur<'_>) -> R<Value>) -> R<Value> {
    let n = c.count(size)?;
    Ok(Value::Array((0..n).map(|_| item(c)).collect::<R<_>>()?))
}

/// The value of a dictionary entry of the given type, as `ON_BinaryArchive` writes it.
fn entry_value(kind: i32, c: &mut Cur<'_>, depth: usize) -> R<Value> {
    let i16v = |c: &mut Cur<'_>| c.array::<2>().map(i16::from_le_bytes);
    let u16v = |c: &mut Cur<'_>| c.array::<2>().map(u16::from_le_bytes);
    let f32v = |c: &mut Cur<'_>| c.array::<4>().map(f32::from_le_bytes);
    Ok(match kind {
        1 => Value::Bool(c.bool()?),
        2 => Value::Int(c.u8()? as i64),
        3 => Value::Int(c.u8()? as i8 as i64),
        4 => Value::Int(i16v(c)? as i64),
        5 => Value::Int(u16v(c)? as i64),
        6 => Value::Int(c.i32()? as i64),
        7 => Value::Int(c.u32()? as i64),
        8 => Value::Int(c.i64()?),
        9 => Value::Number(f32v(c)? as f64),
        10 => Value::Number(c.f64()?),
        11 => Value::Uuid(c.uuid()?),
        12 => Value::String(c.wstring()?),
        13 => array(c, 1, |c| Ok(Value::Bool(c.bool()?)))?,
        14 => array(c, 1, |c| Ok(Value::Int(c.u8()? as i64)))?,
        15 => array(c, 1, |c| Ok(Value::Int(c.u8()? as i8 as i64)))?,
        16 => array(c, 2, |c| Ok(Value::Int(i16v(c)? as i64)))?,
        17 => array(c, 4, |c| Ok(Value::Int(c.i32()? as i64)))?,
        18 => array(c, 4, |c| Ok(Value::Number(f32v(c)? as f64)))?,
        19 => array(c, 8, |c| Ok(Value::Number(c.f64()?)))?,
        20 => array(c, 16, |c| Ok(Value::Uuid(c.uuid()?)))?,
        21 => array(c, 4, |c| Ok(Value::String(c.wstring()?)))?,
        22 => Value::Color(c.color()?),
        23 | 27 => ints(c, 2)?,
        24 | 28 => floats(c, 2)?,
        25 => ints(c, 4)?,
        26 => floats(c, 4)?,
        30 | 31 | 34 => doubles(c, 2)?,
        32 | 35 => doubles(c, 3)?,
        33 | 38 => doubles(c, 4)?,
        36 | 37 | 41 => doubles(c, 6)?,
        39 | 40 => doubles(c, 16)?,
        42 | 43 => floats(c, 3)?,
        44 => Value::Dictionary(Box::new(dictionary(c, depth + 1)?)),
        45 | 47 => match c.object()? {
            Some(o) => Value::Object { class: o.class, bytes: o.data.data.len() },
            None => Value::Object { class: Uuid::default(), bytes: 0 },
        },
        _ => return err("a dictionary entry of an unknown type"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use threedm_core::write::*;

    #[test]
    fn dictionaries_are_read_with_their_types_and_nesting() {
        let w = W { big: true };
        let entry = |kind: i32, key: &str, value: &[u8]| w.chunk(TCODE_DICTIONARY_ENTRY, &cat(&[&i32b(kind), &wstr(key), value]));
        let dict = |entries: &[Vec<u8>]| {
            let id = w.versioned(TCODE_DICTIONARY_ID, 1, 0, &cat(&[&Uuid::parse("21EE7933-1E2D-4047-869E-6BDBF986EA11").0, &i32b(1), &wstr("")]));
            let mut body = id;
            body.extend(entries.concat());
            body.extend(w.short(TCODE_DICTIONARY_END, 0));
            w.versioned(TCODE_DICTIONARY, 1, 0, &body)
        };
        let inner = dict(&[entry(6, "level", &i32b(2))]);
        let outer = dict(&[
            entry(1, "flag", &[1]),
            entry(10, "length", &f64s(&[2.5])),
            entry(12, "name", &wstr("deur")),
            entry(32, "where", &f64s(&[1.0, 2.0, 3.0])),
            entry(17, "counts", &cat(&[&i32b(2), &i32b(4), &i32b(-5)])),
            entry(44, "nested", &inner),
            entry(99, "future", &[1, 2, 3]),
        ]);
        let c = Cur { data: &outer, pos: 0, v: Versions { archive: 90, opennurbs: 0 } };
        let Some(Decoded::Dictionary(d)) = decode(Uuid::default(), c) else { panic!("not a dictionary") };
        let values: Vec<&Value> = d.entries.iter().map(|e| &e.value).collect();
        assert_eq!(values[0], &Value::Bool(true));
        assert_eq!(values[1], &Value::Number(2.5));
        assert_eq!(values[2], &Value::String("deur".into()));
        assert_eq!(values[3], &Value::Numbers(vec![1.0, 2.0, 3.0]));
        assert_eq!(values[4], &Value::Array(vec![Value::Int(4), Value::Int(-5)]));
        let Value::Dictionary(n) = values[5] else { panic!("not nested") };
        assert_eq!(n.entries[0].value, Value::Int(2));
        assert_eq!(values[6], &Value::Raw(vec![1, 2, 3]));
    }

    #[test]
    fn xml_is_found_after_its_length() {
        let xml = b"<xml><a/></xml>\r\n";
        let bytes = cat(&[&i32b(4), &i32b(xml.len() as i32), xml]);
        assert_eq!(text(&bytes).as_deref(), Some("<xml><a/></xml>"));
        assert_eq!(text(&[1, 2, 3, 4]), None);
    }
}
