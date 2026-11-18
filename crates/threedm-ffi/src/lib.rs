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
//!   the number of layers, materials, bitmaps, blocks, lights and objects, the linetypes (with
//!   their Rhino 8 width, cap, join and taper) and section styles, the views (layouts
//!   with their `page`: paper in mm and page number) and the layouts alone, the objects by
//!   kind, and the records that could not be read;
//! - `{"query": "layers"}`, `{"query": "materials"}`, `{"query": "bitmaps"}`,
//!   `{"query": "blocks"}`, `{"query": "lights"}`: the tables (materials with their colours,
//!   textures and physically based parameters, embedded bitmaps by name, file and byte count);
//! - `{"query": "objects", "limit": 10, "offset": 0}`: objects with their id, name, kind, layer,
//!   attributes (section source, style and own style, clipping label) and user text, and of a
//!   curve its `stroke` (linetype width, cap, join, taper, print width), `{"total", "objects"}`;
//! - `{"query": "object", "id": "…"}`: one object by its id, with its geometry: meshes
//!   (`positions`, triangle `indices`, optional `normals`, `texture_coordinates` and `colors`),
//!   `polylines`, `points` or the block `definition` and `xform` (column-major 4×4) of an instance,
//!   or of a layout's detail its `boundary` on the page, the model `view` it shows, its `scale`
//!   (page per model length) and whether it is `locked`; objects on a layout have `space` 2 and
//!   the layout's id as `viewport`; a text, dimension or leader has its `font` (family, face,
//!   weight, bold, italic: of its first character);
//! - `{"query": "settings"}`: the document settings: earth anchor point, model base point and
//!   URL, named and view construction planes, render, analysis and custom mesh settings, grid
//!   defaults, render settings, the current table entries, the plug-ins; the document user
//!   text, named layer states, snapshots, dimension styles with their fonts, the font table and
//!   the plug-in tables (by plug-in, with their size);
//! - `{"query": "history"}`: the history records: command, version, the objects they were made
//!   from and made, their values by id and type.

use std::ffi::{c_char, CStr, CString};

mod settings;

use serde_json::{json, Value};
use threedm_read::{Document, File, Geometry, Light, LightKind, LineCap, LineJoin, Linetype, Material, Mesh, RhinoObject, SectionStyle, Stroke};

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
        Geometry::Meshes(_, kind) | Geometry::Lines(_, kind) | Geometry::Drawing { kind, .. } => kind.to_string(),
        Geometry::Points { kind, .. } => kind.to_string(),
        Geometry::Instance { .. } => "Block instance".into(),
        Geometry::ClippingPlane(_) => "Clipping plane".into(),
        Geometry::Detail { .. } => "Detail".into(),
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
        "url": a.url,
        "groups": a.groups,
        "viewport": (!a.viewport.is_nil()).then(|| a.viewport.to_string()),
        "display_order": a.display_order,
        "plot_color": a.plot_color,
        "plot_weight": a.plot_weight,
        "linetype_scale": a.linetype_scale,
        "custom_linetype": a.custom_linetype,
        "hatch_background": a.hatch_background,
        "font": o.font.as_ref().map(settings::font_json),
        "hatch_boundary": a.hatch_boundary,
        "custom_linetype_style": a.custom_linetype_style.as_ref().map(linetype_json),
        "section_source": a.section_source,
        "section_style": a.section_style,
        "custom_section_style": a.custom_section_style.as_ref().map(section_style_json),
        "clipping_label": a.clipping_label,
        "stroke": o.stroke.as_ref().map(stroke_json),
        "user_text": user,
    })
}

fn cap(c: LineCap) -> &'static str {
    match c {
        LineCap::Round => "round",
        LineCap::Flat => "flat",
        LineCap::Square => "square",
    }
}

fn join(j: LineJoin) -> &'static str {
    match j {
        LineJoin::Round => "round",
        LineJoin::Miter => "miter",
        LineJoin::Bevel => "bevel",
    }
}

fn linetype_json(l: &Linetype) -> Value {
    json!({
        "index": l.index, "id": l.id.to_string(), "name": l.name, "segments": l.segments,
        "width": l.width, "width_unit": l.width_unit, "cap": cap(l.cap), "join": join(l.join),
        "taper": l.taper, "always_model_distances": l.always_model_distances,
    })
}

fn stroke_json(s: &Stroke) -> Value {
    json!({
        "linetype": s.linetype, "width": s.width, "width_unit": s.width_unit, "model_width": s.model_width,
        "cap": cap(s.cap), "join": join(s.join), "taper": s.taper, "print_width": s.print_width, "print_color": s.print_color,
    })
}

fn section_style_json(s: &SectionStyle) -> Value {
    json!({
        "index": s.index, "id": s.id.to_string(), "name": s.name,
        "background_fill_mode": s.background_fill_mode, "background_fill_color": s.background_fill_color,
        "background_fill_print_color": s.background_fill_print_color,
        "boundary_visible": s.boundary_visible, "boundary_color": s.boundary_color, "boundary_print_color": s.boundary_print_color,
        "boundary_width_scale": s.boundary_width_scale, "boundary_print_width": s.boundary_print_width,
        "boundary_linetype_index": s.boundary_linetype_index, "boundary_linetype": s.boundary_linetype.as_ref().map(linetype_json),
        "fill_rule": s.fill_rule, "hatch_index": s.hatch_index, "hatch_scale": s.hatch_scale, "hatch_rotation": s.hatch_rotation,
        "hatch_color": s.hatch_color, "hatch_print_color": s.hatch_print_color, "hatch_print_width": s.hatch_print_width,
    })
}

fn mesh_json(m: &Mesh) -> Value {
    let indices: Vec<u32> = m.faces.iter().flat_map(|f| if f[2] == f[3] { vec![f[0], f[1], f[2]] } else { vec![f[0], f[1], f[2], f[0], f[2], f[3]] }).collect();
    json!({
        "positions": m.positions.iter().flatten().collect::<Vec<_>>(),
        "indices": indices,
        "normals": m.normals.as_ref().map(|n| n.iter().flatten().copied().collect::<Vec<_>>()),
        "colors": m.colors.as_ref().map(|c| c.iter().flatten().copied().collect::<Vec<_>>()),
        "texture_coordinates": m.texture_coordinates.as_ref().map(|t| t.iter().flatten().copied().collect::<Vec<_>>()),
        "color": m.color,
        "computed": m.computed,
    })
}

fn material_json(m: &Material) -> Value {
    let textures: Vec<Value> = m
        .textures
        .iter()
        .map(|t| json!({"file": t.file, "type": t.kind, "enabled": t.enabled, "mode": t.mode, "amount": t.amount, "uvw": t.uvw.to_cols_array(), "mapping_channel": t.mapping_channel}))
        .collect();
    let pbr = m.pbr.as_ref().map(|p| {
        json!({
            "base_color": p.base_color, "brdf": p.brdf, "subsurface": p.subsurface,
            "subsurface_scattering_color": p.subsurface_scattering_color, "subsurface_scattering_radius": p.subsurface_scattering_radius,
            "metallic": p.metallic, "specular": p.specular, "specular_tint": p.specular_tint, "roughness": p.roughness,
            "anisotropic": p.anisotropic, "anisotropic_rotation": p.anisotropic_rotation, "sheen": p.sheen, "sheen_tint": p.sheen_tint,
            "clearcoat": p.clearcoat, "clearcoat_roughness": p.clearcoat_roughness, "opacity_ior": p.opacity_ior,
            "opacity": p.opacity, "opacity_roughness": p.opacity_roughness, "emission": p.emission, "alpha": p.alpha,
        })
    });
    json!({
        "index": m.index, "name": m.name,
        "ambient": m.ambient, "diffuse": m.diffuse, "emission": m.emission, "specular": m.specular,
        "reflection": m.reflection, "transparent": m.transparent,
        "shine": m.shine, "reflectivity": m.reflectivity, "index_of_refraction": m.index_of_refraction,
        "transparency": m.transparency, "textures": textures, "pbr": pbr,
    })
}

fn polylines(lines: &[Vec<glam::DVec3>]) -> Vec<Vec<[f64; 3]>> {
    lines.iter().map(|l| l.iter().map(|p| [p.x, p.y, p.z]).collect()).collect()
}

fn geometry_json(g: &Geometry<'_>) -> Value {
    match g {
        Geometry::Meshes(meshes, _) => json!({"meshes": meshes.iter().map(mesh_json).collect::<Vec<_>>()}),
        Geometry::Lines(lines, _) => json!({"polylines": polylines(lines)}),
        Geometry::Drawing { lines, meshes, .. } => json!({"polylines": polylines(lines), "meshes": meshes.iter().map(mesh_json).collect::<Vec<_>>()}),
        Geometry::Points { points, colors, .. } => {
            let xyz: Vec<f64> = points.chunks_exact(8).map(|b| f64::from_le_bytes(b.try_into().unwrap())).collect();
            json!({"points": xyz, "colors": colors.map(|c| c.to_vec())})
        }
        Geometry::Instance { definition, xform } => json!({"definition": definition.to_string(), "xform": xform.to_cols_array()}),
        Geometry::ClippingPlane(p) => json!({
            "corners": p.corners.iter().map(|c| c.to_array()).collect::<Vec<_>>(),
            "origin": p.origin.to_array(),
            "normal": p.normal.to_array(),
            "enabled": p.enabled,
            "viewports": p.viewports.iter().map(|v| v.to_string()).collect::<Vec<_>>(),
            "depth": p.depth,
            "selection": p.selection.as_ref().map(|s| json!({"objects": s.objects.iter().map(|o| o.to_string()).collect::<Vec<_>>(), "layers": s.layers, "except": s.except})),
        }),
        Geometry::Detail { boundary, view, scale, locked } => json!({
            "boundary": boundary.iter().map(|p| p.to_array()).collect::<Vec<_>>(),
            "view": view_json(view),
            "scale": scale,
            "locked": locked,
        }),
        Geometry::Skipped(why) => json!({"left_out": why}),
    }
}

fn light_json(l: &Light) -> Value {
    let kind = match l.kind {
        LightKind::Directional => "directional",
        LightKind::Point => "point",
        LightKind::Spot => "spot",
        LightKind::Linear => "linear",
        LightKind::Rectangular => "rectangular",
        LightKind::Ambient => "ambient",
        LightKind::Unknown => "unknown",
    };
    json!({
        "id": l.id.to_string(),
        "name": l.name,
        "layer": l.layer,
        "visible": l.visible,
        "on": l.on,
        "kind": kind,
        "camera": l.camera,
        "location": l.location.to_array(),
        "direction": l.direction.to_array(),
        "length": l.length.to_array(),
        "width": l.width.to_array(),
        "ambient": l.ambient,
        "diffuse": l.diffuse,
        "specular": l.specular,
        "intensity": l.intensity,
        "watts": l.watts,
        "spot_angle": l.spot_angle,
        "spot_exponent": l.spot_exponent,
        "hotspot": l.hotspot,
        "attenuation": l.attenuation.to_array(),
        "shadow_intensity": l.shadow_intensity,
    })
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
                "page_unit": file.page_unit,
                "page_meters_per_unit": file.page_meters,
                "layers": file.layers.len(),
                "materials": file.materials.len(),
                "bitmaps": file.bitmaps.len(),
                "blocks": file.blocks.len(),
                "lights": file.lights.len(),
                "objects": file.objects.len(),
                "views": file.views.iter().map(view_json).collect::<Vec<_>>(),
                "named_views": file.named_views.iter().map(view_json).collect::<Vec<_>>(),
                "layouts": file.views.iter().filter(|v| v.page.is_some()).map(view_json).collect::<Vec<_>>(),
                "groups": file.groups.iter().map(|(i, n)| json!({"index": i, "name": n})).collect::<Vec<_>>(),
                "linetypes": file.linetypes.iter().map(linetype_json).collect::<Vec<_>>(),
                "section_styles": file.section_styles.iter().map(section_style_json).collect::<Vec<_>>(),
                "kinds": kinds,
                "earth_anchor": file.settings.earth_anchor.as_ref().map(|e| json!({"latitude": e.latitude, "longitude": e.longitude, "elevation": e.elevation, "name": e.name})),
                "model_basepoint": file.settings.model_basepoint.to_array(),
                "dim_styles": settings::dim_styles_json(file),
                "history_records": file.history.len(),
                "damaged": file.damaged,
            }))
        }
        "layers" => Ok(file
            .layers
            .iter()
            .map(|l| {
                json!({
                    "index": l.index, "name": l.name, "id": l.id.to_string(), "parent": l.parent.to_string(), "color": l.color, "visible": l.visible,
                    "material": l.material, "linetype": l.linetype, "plot_color": l.plot_color, "plot_weight": l.plot_weight,
                    "section_style": l.section_style, "custom_section_style": l.custom_section_style.as_ref().map(section_style_json),
                })
            })
            .collect()),
        "materials" => Ok(file.materials.iter().map(material_json).collect()),
        "bitmaps" => Ok(file.bitmaps.iter().map(|b| json!({"name": b.name, "file": b.file, "bytes": b.data.as_ref().map(Vec::len)})).collect()),
        "blocks" => Ok(file
            .blocks
            .iter()
            .map(|b| json!({"id": b.id.to_string(), "name": b.name, "objects": b.objects.iter().map(|o| o.to_string()).collect::<Vec<_>>()}))
            .collect()),
        "lights" => Ok(file.lights.iter().map(light_json).collect()),
        "settings" => Ok(settings::settings_json(file)),
        "history" => Ok(file.history.iter().map(settings::history_json).collect()),
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
        other => Err(format!("unknown query {other:?} (info, layers, materials, bitmaps, blocks, lights, objects, object, settings, history)")),
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

fn view_json(v: &threedm_read::View) -> serde_json::Value {
    let page = v.page.map(|p| json!({"width_mm": p.width, "height_mm": p.height, "number": p.number}));
    json!({"id": v.id.to_string(), "name": v.name, "perspective": v.perspective, "location": v.location.to_array(), "direction": v.direction.to_array(), "up": v.up.to_array(), "fov_y": v.fov_y, "height": v.height, "page": page})
}
