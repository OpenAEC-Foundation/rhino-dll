//! JSON of the document settings, fonts, dimension styles, history records and plug-in tables.

use serde_json::{json, Value};
use threedm_read::{ConstructionPlane, EarthAnchor, File, Font, GridDefaults, HistoryData, HistoryRecord, MeshSettings, PlugIn, RenderSettings, Settings};

pub(crate) fn font_json(f: &Font) -> Value {
    json!({"family": f.family, "face": f.face, "postscript": f.postscript, "logfont": f.logfont, "weight": f.weight, "style": f.style, "bold": f.bold, "italic": f.italic, "underlined": f.underlined, "strikeout": f.strikeout})
}

fn earth_anchor_json(e: &EarthAnchor) -> Value {
    json!({
        "latitude": e.latitude, "longitude": e.longitude, "elevation": e.elevation, "coordinate_system": e.coordinate_system,
        "model_point": e.model_point.to_array(), "model_north": e.model_north.to_array(), "model_east": e.model_east.to_array(),
        "id": e.id.to_string(), "name": e.name, "description": e.description, "url": e.url, "url_tag": e.url_tag,
    })
}

fn cplane_json(p: &ConstructionPlane) -> Value {
    json!({
        "name": p.name, "origin": p.origin.to_array(), "x": p.x.to_array(), "y": p.y.to_array(), "z": p.z.to_array(),
        "grid_spacing": p.grid_spacing, "snap_spacing": p.snap_spacing, "grid_line_count": p.grid_line_count,
        "thick_frequency": p.thick_frequency, "depth_buffered": p.depth_buffered,
    })
}

fn mesh_json(m: &MeshSettings) -> Value {
    json!({
        "compute_curvature": m.compute_curvature, "simple_planes": m.simple_planes, "refine": m.refine, "jagged_seams": m.jagged_seams,
        "tolerance": m.tolerance, "min_edge_length": m.min_edge_length, "max_edge_length": m.max_edge_length,
        "grid_aspect_ratio": m.grid_aspect_ratio, "grid_min_count": m.grid_min_count, "grid_max_count": m.grid_max_count,
        "grid_angle": m.grid_angle, "grid_amplification": m.grid_amplification, "refine_angle": m.refine_angle,
        "face_type": m.face_type, "texture_range": m.texture_range, "custom": m.custom, "relative_tolerance": m.relative_tolerance,
        "mesher": m.mesher, "custom_enabled": m.custom_enabled,
    })
}

fn grid_json(g: &GridDefaults) -> Value {
    json!({"grid_spacing": g.grid_spacing, "snap_spacing": g.snap_spacing, "grid_line_count": g.grid_line_count, "thick_frequency": g.thick_frequency, "show_grid": g.show_grid, "show_axes": g.show_axes, "show_world_axes": g.show_world_axes})
}

fn render_json(r: &RenderSettings) -> Value {
    json!({
        "custom_image_size": r.custom_image_size, "image_width": r.image_width, "image_height": r.image_height, "image_dpi": r.image_dpi,
        "image_unit": r.image_unit, "ambient": r.ambient, "background_style": r.background_style, "background_color": r.background_color,
        "background_bottom": r.background_bottom, "background_bitmap": r.background_bitmap, "use_hidden_lights": r.use_hidden_lights,
        "depth_cue": r.depth_cue, "flat_shade": r.flat_shade, "render_backfaces": r.render_backfaces, "render_points": r.render_points,
        "render_curves": r.render_curves, "render_isoparams": r.render_isoparams, "render_mesh_edges": r.render_mesh_edges,
        "render_annotations": r.render_annotations, "scale_background": r.scale_background, "transparent_background": r.transparent_background,
        "antialias": r.antialias, "shadowmap": r.shadowmap, "shadowmap_width": r.shadowmap_width, "shadowmap_height": r.shadowmap_height,
        "shadowmap_offset": r.shadowmap_offset, "rendering_source": r.rendering_source, "specific_viewport": r.specific_viewport,
        "named_view": r.named_view, "snapshot": r.snapshot, "force_viewport_aspect_ratio": r.force_viewport_aspect_ratio,
    })
}

fn plugin_json(p: &PlugIn) -> Value {
    json!({
        "id": p.id.to_string(), "kind": p.kind, "name": p.name, "version": p.version, "filename": p.filename,
        "organization": p.organization, "address": p.address, "country": p.country, "phone": p.phone, "email": p.email,
        "website": p.website, "update_url": p.update_url, "fax": p.fax, "platform": p.platform, "sdk_version": p.sdk_version,
        "sdk_service_release": p.sdk_service_release,
    })
}

/// The settings in full (`{"query": "settings"}`), with the document user text, layer states,
/// snapshots, dimension styles and the font table.
pub(crate) fn settings_json(file: &File<'_>) -> Value {
    let s: &Settings = &file.settings;
    let c = &s.current;
    let id = |u: &threedm_read::Uuid| u.to_string();
    json!({
        "earth_anchor": s.earth_anchor.as_ref().map(earth_anchor_json),
        "model_basepoint": s.model_basepoint.to_array(),
        "model_url": s.model_url,
        "named_cplanes": s.named_cplanes.iter().map(cplane_json).collect::<Vec<_>>(),
        "view_cplanes": s.view_cplanes.iter().map(|(v, p)| json!({"viewport": v.to_string(), "cplane": cplane_json(p)})).collect::<Vec<_>>(),
        "render_mesh": s.render_mesh.as_ref().map(mesh_json),
        "analysis_mesh": s.analysis_mesh.as_ref().map(mesh_json),
        "custom_render_mesh": s.custom_render_mesh.as_ref().map(mesh_json),
        "grid": s.grid.as_ref().map(grid_json),
        "render": s.render.as_ref().map(render_json),
        "current": {
            "layer_index": c.layer_index, "layer": id(&c.layer), "material_index": c.material_index, "material": id(&c.material),
            "material_source": c.material_source, "color": c.color, "color_source": c.color_source, "plot_color": c.plot_color,
            "plot_color_source": c.plot_color_source, "linetype_index": c.linetype_index, "linetype": id(&c.linetype),
            "linetype_source": c.linetype_source, "text_style_index": c.text_style_index, "text_style": id(&c.text_style),
            "dimension_style_index": c.dimension_style_index, "dimension_style": id(&c.dimension_style),
            "hatch_pattern": id(&c.hatch_pattern), "wire_density": c.wire_density,
        },
        "linetype_display_scale": s.linetype_display_scale,
        "active_view": id(&s.active_view),
        "save_texture_bitmaps": s.save_texture_bitmaps,
        "idef_link_update": s.idef_link_update,
        "plugins": s.plugins.iter().map(plugin_json).collect::<Vec<_>>(),
        "document_user_text": file.document_user_text.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>(),
        "layer_states": file.layer_states.iter().map(|l| json!({
            "name": l.name, "current": l.current, "current_id": l.current_id.to_string(),
            "layers": l.layers.iter().map(|y| json!({"name": y.name, "id": y.id.to_string(), "visible": y.visible, "color": y.color})).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "snapshots": file.snapshots.iter().map(|s| json!({"name": s.name, "id": s.id, "xml": s.xml})).collect::<Vec<_>>(),
        "dim_styles": dim_styles_json(file),
        "fonts": file.fonts.iter().map(|(i, f)| json!({"index": i, "font": font_json(f)})).collect::<Vec<_>>(),
        "user_tables": file.user_tables.iter().map(|t| json!({"plugin": t.plugin.to_string(), "name": t.name, "goo": t.goo, "archive_version": t.archive_version, "opennurbs_version": t.opennurbs_version, "bytes": t.data.len()})).collect::<Vec<_>>(),
    })
}

/// The dimension styles with their fonts.
pub(crate) fn dim_styles_json(file: &File<'_>) -> Value {
    file.dim_styles.iter().map(|d| json!({"id": d.id.to_string(), "index": d.index, "name": d.name, "font": d.font.as_ref().map(font_json)})).collect()
}

fn history_data_json(d: &HistoryData) -> Value {
    match d {
        HistoryData::Bools(v) => json!(v),
        HistoryData::Ints(v) => json!(v),
        HistoryData::Doubles(v) => json!(v),
        HistoryData::Colors(v) => json!(v),
        HistoryData::Points(v) | HistoryData::Vectors(v) => json!(v.iter().map(|p| p.to_array()).collect::<Vec<_>>()),
        HistoryData::Xforms(v) => json!(v.iter().map(|m| m.to_cols_array()).collect::<Vec<_>>()),
        HistoryData::Strings(v) => json!(v),
        HistoryData::Uuids(v) | HistoryData::Objects(v) => json!(v.iter().map(|u| u.to_string()).collect::<Vec<_>>()),
        HistoryData::Raw(_, bytes) => json!({"bytes": bytes.len()}),
    }
}

/// The history records (`{"query": "history"}`).
pub(crate) fn history_json(h: &HistoryRecord) -> Value {
    json!({
        "id": h.id.to_string(), "command": h.command.to_string(), "version": h.version, "kind": h.kind,
        "antecedents": h.antecedents.iter().map(|u| u.to_string()).collect::<Vec<_>>(),
        "descendants": h.descendants.iter().map(|u| u.to_string()).collect::<Vec<_>>(),
        "copy_on_replace": h.copy_on_replace,
        "values": h.values.iter().map(|v| json!({"id": v.id, "type": v.data.kind(), "value": history_data_json(&v.data)})).collect::<Vec<_>>(),
    })
}
