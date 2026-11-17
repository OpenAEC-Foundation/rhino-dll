//! C ABI of the 3dm reader (`threedm.dll`).
//!
//! ```c
//! ThreedmDocument *doc = threedm_open("model.3dm");      // NULL on failure
//! char *json = threedm_query(doc, "{\"query\": \"objects\", \"limit\": 10}");
//! /* ... */
//! threedm_free(json);
//! threedm_close(doc);
//! ```
//!
//! `threedm_query` takes a JSON request and returns a JSON answer, `{"ok": true, "result": …}` or
//! `{"ok": false, "error": "…"}`; the caller frees it with `threedm_free`. Requests:
//! - `{"query": "info"}`: the Rhino version, the application that saved the file, its units,
//!   the number of layers, materials, blocks and objects, the objects by kind, and the records
//!   that could not be read;
//! - `{"query": "layers"}`, `{"query": "materials"}`, `{"query": "blocks"}`: the tables;
//! - `{"query": "objects", "limit": 10, "offset": 0}`: objects with their id, name, kind, layer,
//!   attributes and user text, `{"total", "objects"}`;
//! - `{"query": "object", "id": "…"}`: one object by its id, with its geometry: meshes
//!   (`positions`, triangle `indices`, optional `normals` and `colors`), `polylines`, `points`
//!   or the block `definition` and `xform` (column-major 4×4) of an instance.

use std::ffi::{c_char, CStr, CString};

use serde_json::{json, Value};
use threedm_read::{Document, File, Geometry, Mesh, RhinoObject};

/// An open document behind the C ABI.
pub struct ThreedmDocument(Document);

/// Answers one JSON request on a document (the safe core of `threedm_query`).
pub fn query(doc: &Document, request: &str) -> Value {
    match answer(doc.file(), request) {
        Ok(v) => json!({"ok": true, "result": v}),
        Err(e) => json!({"ok": false, "error": e}),
    }
}

/// What an object is: its kind as Rhino names it ("Polysurface", "Curve", ..), or why it was
/// left out.
fn kind(o: &RhinoObject<'_>) -> String {
    match &o.geometry {
        Geometry::Meshes(_, kind) | Geometry::Lines(_, kind) => kind.to_string(),
        Geometry::Points { kind, .. } => kind.to_string(),
        Geometry::Instance { .. } => "Block instance".into(),
        Geometry::Skipped(why) => format!("left out: {why}"),
    }
}

fn object_summary(o: &RhinoObject<'_>) -> Value {
    let a = &o.attributes;
    let user: serde_json::Map<String, Value> = o.user.iter().map(|(k, v)| (k.clone(), Value::from(v.as_str()))).collect();
    json!({
        "id": a.id.to_string(),
        "name": a.name,
        "kind": kind(o),
        "layer": a.layer,
        "material": a.material,
        "color": a.color,
        "color_source": a.color_source,
        "visible": a.visible,
        "mode": a.mode,
        "space": a.space,
        "user_text": user,
    })
}

fn mesh_json(m: &Mesh) -> Value {
    let indices: Vec<u32> = m.faces.iter().flat_map(|f| if f[2] == f[3] { vec![f[0], f[1], f[2]] } else { vec![f[0], f[1], f[2], f[0], f[2], f[3]] }).collect();
    json!({
        "positions": m.positions.iter().flatten().collect::<Vec<_>>(),
        "indices": indices,
        "normals": m.normals.as_ref().map(|n| n.iter().flatten().copied().collect::<Vec<_>>()),
        "colors": m.colors.as_ref().map(|c| c.iter().flatten().copied().collect::<Vec<_>>()),
        "color": m.color,
        "computed": m.computed,
    })
}

fn geometry_json(g: &Geometry<'_>) -> Value {
    match g {
        Geometry::Meshes(meshes, _) => json!({"meshes": meshes.iter().map(mesh_json).collect::<Vec<_>>()}),
        Geometry::Lines(lines, _) => json!({"polylines": lines.iter().map(|l| l.iter().map(|p| [p.x, p.y, p.z]).collect::<Vec<_>>()).collect::<Vec<_>>()}),
        Geometry::Points { points, colors, .. } => {
            let xyz: Vec<f64> = points.chunks_exact(8).map(|b| f64::from_le_bytes(b.try_into().unwrap())).collect();
            json!({"points": xyz, "colors": colors.map(|c| c.to_vec())})
        }
        Geometry::Instance { definition, xform } => json!({"definition": definition.to_string(), "xform": xform.to_cols_array()}),
        Geometry::Skipped(why) => json!({"left_out": why}),
    }
}

fn answer(file: &File<'_>, request: &str) -> Result<Value, String> {
    let req: Value = serde_json::from_str(request).map_err(|e| format!("request is not JSON: {e}"))?;
    let q = req.get("query").and_then(Value::as_str).ok_or("request has no \"query\"")?;
    match q {
        "info" => {
            let mut kinds = std::collections::BTreeMap::<String, usize>::new();
            for o in &file.objects {
                *kinds.entry(kind(o)).or_default() += 1;
            }
            Ok(json!({
                "archive_version": file.versions.archive,
                "opennurbs_version": file.versions.opennurbs,
                "application": file.application,
                "unit": file.unit,
                "meters_per_unit": file.meters,
                "layers": file.layers.len(),
                "materials": file.materials.len(),
                "blocks": file.blocks.len(),
                "objects": file.objects.len(),
                "kinds": kinds,
                "damaged": file.damaged,
            }))
        }
        "layers" => Ok(file
            .layers
            .iter()
            .map(|l| json!({"index": l.index, "name": l.name, "id": l.id.to_string(), "parent": l.parent.to_string(), "color": l.color, "visible": l.visible, "material": l.material}))
            .collect()),
        "materials" => Ok(file.materials.iter().map(|m| json!({"index": m.index, "name": m.name, "diffuse": m.diffuse, "transparency": m.transparency})).collect()),
        "blocks" => Ok(file
            .blocks
            .iter()
            .map(|b| json!({"id": b.id.to_string(), "name": b.name, "objects": b.objects.iter().map(|o| o.to_string()).collect::<Vec<_>>()}))
            .collect()),
        "objects" => {
            let limit = req.get("limit").and_then(Value::as_u64).map_or(usize::MAX, |n| n as usize);
            let offset = req.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
            let objects: Vec<Value> = file.objects.iter().skip(offset).take(limit).map(object_summary).collect();
            Ok(json!({"total": file.objects.len(), "objects": objects}))
        }
        "object" => {
            let id = req.get("id").and_then(Value::as_str).ok_or("\"object\" needs an \"id\" (a uuid)")?;
            let o = file.objects.iter().find(|o| o.attributes.id.to_string().eq_ignore_ascii_case(id)).ok_or_else(|| format!("no object {id}"))?;
            let mut v = object_summary(o);
            v["geometry"] = geometry_json(&o.geometry);
            Ok(v)
        }
        other => Err(format!("unknown query {other:?} (info, layers, materials, blocks, objects, object)")),
    }
}

fn c_string(v: &Value) -> *mut c_char {
    // serde_json escapes control characters, so the text has no interior NUL
    CString::new(v.to_string()).map_or(std::ptr::null_mut(), CString::into_raw)
}

/// Opens a `.3dm` file (UTF-8 path). Returns NULL when the path is NULL or the file cannot be
/// read.
///
/// # Safety
/// `path` must be NULL or a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn threedm_open(path: *const c_char) -> *mut ThreedmDocument {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(p) = CStr::from_ptr(path).to_str() else { return std::ptr::null_mut() };
    match std::panic::catch_unwind(|| Document::open(p)) {
        Ok(Ok(d)) => Box::into_raw(Box::new(ThreedmDocument(d))),
        _ => std::ptr::null_mut(),
    }
}

/// Answers a JSON request (see the crate documentation). The result is freed with
/// `threedm_free`; NULL only when the answer could not be allocated.
///
/// # Safety
/// `doc` must be NULL or a pointer from `threedm_open` not yet closed; `request` NULL or a
/// NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn threedm_query(doc: *const ThreedmDocument, request: *const c_char) -> *mut c_char {
    if doc.is_null() || request.is_null() {
        return c_string(&json!({"ok": false, "error": "null document or request"}));
    }
    let Ok(r) = CStr::from_ptr(request).to_str() else { return c_string(&json!({"ok": false, "error": "request is not UTF-8"})) };
    let d = &(*doc).0;
    let v = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| query(d, r))).unwrap_or_else(|_| json!({"ok": false, "error": "internal error"}));
    c_string(&v)
}

/// Frees a string returned by `threedm_query`.
///
/// # Safety
/// `s` must be NULL or a pointer from `threedm_query` not yet freed.
#[no_mangle]
pub unsafe extern "C" fn threedm_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

/// Closes a document from `threedm_open`.
///
/// # Safety
/// `doc` must be NULL or a pointer from `threedm_open` not yet closed.
#[no_mangle]
pub unsafe extern "C" fn threedm_close(doc: *mut ThreedmDocument) {
    if !doc.is_null() {
        drop(Box::from_raw(doc));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_answer_in_json() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../test-models/drafting.3dm");
        let doc = Document::open(path).unwrap();
        let info = query(&doc, r#"{"query": "info"}"#);
        assert_eq!(info["ok"], true);
        assert_eq!(info["result"]["archive_version"], 90);
        assert_eq!(info["result"]["kinds"]["Text"], 6);
        let objects = query(&doc, r#"{"query": "objects", "limit": 2}"#);
        let first = &objects["result"]["objects"][0];
        let id = first["id"].as_str().unwrap();
        let one = query(&doc, &format!(r#"{{"query": "object", "id": "{id}"}}"#));
        assert_eq!(one["result"]["id"], id);
        assert!(one["result"]["geometry"].is_object());
        assert_eq!(query(&doc, r#"{"query": "nothing"}"#)["ok"], false);
        assert_eq!(query(&doc, "not json")["ok"], false);
    }

    #[test]
    fn the_c_functions_take_null() {
        unsafe {
            assert!(threedm_open(std::ptr::null()).is_null());
            let answer = threedm_query(std::ptr::null(), std::ptr::null());
            assert!(CStr::from_ptr(answer).to_str().unwrap().contains("null document"));
            threedm_free(answer);
            threedm_close(std::ptr::null_mut());
        }
    }
}
