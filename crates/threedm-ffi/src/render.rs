//! The rendering data as JSON: render settings, render content, decals and mesh modifiers.

use serde_json::{json, Map, Value};
use threedm_read::{Decal, MeshModifiers, RenderContent, RenderSettings, RenderValue, SafeFrameFrame, Uuid};

fn id(u: &Uuid) -> Value {
    if u.is_nil() { Value::Null } else { Value::from(u.to_string()) }
}

/// A parameter value: booleans, numbers and text as they are, uuids as text, colours and
/// vectors as arrays, a typeless empty value as null.
fn value(v: &RenderValue) -> Value {
    match v {
        RenderValue::Bool(b) => json!(b),
        RenderValue::Integer(i) => json!(i),
        RenderValue::Number(n) => json!(n),
        RenderValue::Text(t) => json!(t),
        RenderValue::Uuid(u) => json!(u.to_string()),
        RenderValue::Color(c) => json!(c),
        RenderValue::Numbers(n) => json!(n),
        RenderValue::Null => Value::Null,
    }
}

fn parameters(p: &[(String, RenderValue)]) -> Value {
    Value::Object(p.iter().map(|(n, v)| (n.clone(), value(v))).collect::<Map<_, _>>())
}

fn frame(f: &SafeFrameFrame) -> Value {
    json!({"on": f.on, "linked": f.linked, "x_scale": f.x_scale, "y_scale": f.y_scale})
}

/// The render settings, with the ground plane, sun, skylight and the other parts.
pub fn settings(s: &RenderSettings) -> Value {
    let (g, sun, sky) = (&s.ground_plane, &s.sun, &s.skylight);
    let base = json!({
        "custom_image_size": s.custom_image_size,
        "image_width": s.image_width,
        "image_height": s.image_height,
        "image_dpi": s.image_dpi,
        "image_unit": s.image_unit,
        "ambient": s.ambient,
        "background_style": s.background_style,
        "background_color": s.background_color,
        "background_bottom_color": s.background_bottom_color,
        "wallpaper": s.wallpaper,
        "use_hidden_lights": s.use_hidden_lights,
        "depth_cue": s.depth_cue,
        "flat_shade": s.flat_shade,
        "render_backfaces": s.render_backfaces,
        "render_points": s.render_points,
        "render_curves": s.render_curves,
        "render_isoparams": s.render_isoparams,
        "render_mesh_edges": s.render_mesh_edges,
        "render_annotations": s.render_annotations,
        "scale_background_to_fit": s.scale_background_to_fit,
        "transparent_background": s.transparent_background,
        "antialias": s.antialias,
        "shadowmap": s.shadowmap,
        "shadowmap_width": s.shadowmap_width,
        "shadowmap_height": s.shadowmap_height,
        "shadowmap_offset": s.shadowmap_offset,
        "rendering_source": s.rendering_source,
        "specific_viewport": s.specific_viewport,
        "named_view": s.named_view,
        "snapshot": s.snapshot,
        "force_viewport_aspect_ratio": s.force_viewport_aspect_ratio,
    });
    let parts = json!({
        "background_environment": id(&s.background_environment),
        "reflection_environment_on": s.reflection_environment_on,
        "reflection_environment": id(&s.reflection_environment),
        "ground_plane": {
            "on": g.on, "altitude": g.altitude, "auto_altitude": g.auto_altitude, "shadow_only": g.shadow_only,
            "show_underside": g.show_underside, "material": id(&g.material), "texture_offset": g.texture_offset,
            "texture_size": g.texture_size, "texture_rotation": g.texture_rotation,
            "texture_offset_locked": g.texture_offset_locked, "texture_size_locked": g.texture_size_locked,
        },
        "sun": {
            "on": sun.on, "manual": sun.manual, "azimuth": sun.azimuth, "altitude": sun.altitude, "north": sun.north,
            "latitude": sun.latitude, "longitude": sun.longitude, "time_zone": sun.time_zone,
            "daylight_saving_on": sun.daylight_saving_on, "daylight_saving_minutes": sun.daylight_saving_minutes,
            "year": sun.year, "month": sun.month, "day": sun.day, "hours": sun.hours,
            "intensity": sun.intensity, "shadow_intensity": sun.shadow_intensity, "color": sun.color, "direction": sun.direction,
        },
        "skylight": {"on": sky.on, "shadow_intensity": sky.shadow_intensity, "environment_on": sky.environment_on, "environment": id(&sky.environment)},
        "linear_workflow": {"pre_process": s.linear_workflow.pre_process, "post_process": s.linear_workflow.post_process, "gamma": s.linear_workflow.gamma},
        "dithering": {"on": s.dithering.on, "method": s.dithering.method},
        "safe_frame": {
            "on": s.safe_frame.on, "perspective_only": s.safe_frame.perspective_only, "field_grid_on": s.safe_frame.field_grid_on,
            "live_frame_on": s.safe_frame.live_frame_on, "action_frame": frame(&s.safe_frame.action_frame), "title_frame": frame(&s.safe_frame.title_frame),
        },
        "post_effects": s.post_effects.iter().map(|p| json!({"id": p.id.to_string(), "name": p.name, "stage": p.stage, "on": p.on, "shown": p.shown, "parameters": parameters(&p.parameters)})).collect::<Vec<_>>(),
        "render_channels": {"custom": s.render_channels_custom, "list": s.render_channels.iter().map(Uuid::to_string).collect::<Vec<_>>()},
    });
    let mut out = base;
    if let (Value::Object(out), Value::Object(parts)) = (&mut out, parts) {
        out.extend(parts);
    }
    out
}

/// A render material, environment or texture.
pub fn content(c: &RenderContent) -> Value {
    json!({
        "kind": c.kind,
        "type_name": c.type_name,
        "type_id": c.type_id.to_string(),
        "id": c.id.to_string(),
        "name": c.name,
        "render_engine": c.render_engine.to_string(),
        "plug_in": c.plug_in.to_string(),
        "group": id(&c.group),
        "notes": c.notes,
        "tags": c.tags,
        "hidden": c.hidden,
        "reference": c.reference,
        "auto_delete": c.auto_delete,
        "child_slot": c.child_slot,
        "parent": id(&c.parent),
        "children": c.children.iter().map(Uuid::to_string).collect::<Vec<_>>(),
        "parameters": parameters(&c.parameters),
        "simulation": parameters(&c.simulation),
    })
}

pub fn decal(d: &Decal) -> Value {
    json!({
        "texture": d.texture.to_string(), "mapping": d.mapping, "projection": d.projection,
        "map_to_inside": d.map_to_inside, "transparency": d.transparency, "visible": d.visible,
        "origin": d.origin, "vector_up": d.vector_up, "vector_across": d.vector_across,
        "height": d.height, "radius": d.radius, "horizontal_sweep": d.horizontal_sweep,
        "vertical_sweep": d.vertical_sweep, "uv_bounds": d.uv_bounds,
    })
}

/// The mesh modifiers an object has, by name.
pub fn modifiers(m: &MeshModifiers) -> Value {
    let mut out = Map::new();
    if let Some(d) = &m.displacement {
        out.insert("displacement".into(), json!({
            "on": d.on, "texture": id(&d.texture), "mapping_channel": d.mapping_channel, "black_point": d.black_point,
            "white_point": d.white_point, "sweep_pitch": d.sweep_pitch, "refine_steps": d.refine_steps,
            "refine_sensitivity": d.refine_sensitivity, "face_limit_on": d.face_limit_on, "face_limit": d.face_limit,
            "fairing_on": d.fairing_on, "fairing": d.fairing, "post_weld_angle": d.post_weld_angle,
            "memory_limit": d.memory_limit, "sweep_formula": d.sweep_formula,
        }));
    }
    if let Some(e) = &m.edge_softening {
        out.insert("edge_softening".into(), json!({
            "on": e.on, "softening": e.softening, "chamfer": e.chamfer, "faceted": e.faceted,
            "edge_angle_threshold": e.edge_angle_threshold, "force_softening": e.force_softening,
        }));
    }
    if let Some(t) = &m.thickening {
        out.insert("thickening".into(), json!({"on": t.on, "distance": t.distance, "solid": t.solid, "offset_only": t.offset_only, "both_sides": t.both_sides}));
    }
    if let Some(p) = &m.curve_piping {
        out.insert("curve_piping".into(), json!({"on": p.on, "radius": p.radius, "segments": p.segments, "faceted": p.faceted, "accuracy": p.accuracy, "cap_type": p.cap_type}));
    }
    if let Some(s) = &m.shut_lining {
        let curves: Vec<Value> = s
            .curves
            .iter()
            .map(|c| json!({"id": c.id.to_string(), "enabled": c.enabled, "radius": c.radius, "profile": c.profile, "pull": c.pull, "is_bump": c.is_bump}))
            .collect();
        out.insert("shut_lining".into(), json!({"on": s.on, "faceted": s.faceted, "auto_update": s.auto_update, "force_update": s.force_update, "curves": curves}));
    }
    Value::Object(out)
}
