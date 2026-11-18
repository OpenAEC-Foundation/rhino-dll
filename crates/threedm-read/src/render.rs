//! Rendering data: the render settings (`ON_3dmRenderSettings`, `TCODE_SETTINGS_RENDER`), the
//! RDK document Rhino keeps as the user table of the RDK plug-in (an XML document: render
//! content, environments, ground plane, sun, skylight, linear workflow, dithering, safe frame,
//! post effects, render channels), and of objects their decals (`ON_RdkUserData`) and mesh
//! modifiers (displacement, edge softening, thickening, curve piping, shut lining: XML user
//! data). The XML is read with the small reader of [`crate::xml`].

use crate::xml::{self, Node};
use threedm_core::*;
use threedm_model::{
    CurvePiping, Decal, Displacement, EdgeSoftening, MeshModifiers, PostEffect, RenderContent, RenderSettings, RenderValue, SafeFrameFrame,
    ShutLining, ShutLiningCurve, Thickening,
};

/// `TCODE_SETTINGS_RENDER`: `ON_3dmRenderSettings`.
pub const TCODE_SETTINGS_RENDER: u32 = 0x2000_803D;
/// `TCODE_USER_TABLE`: a plug-in's table; its uuid chunk, then its record.
pub const TCODE_USER_TABLE: u32 = 0x1000_0017;
const TCODE_USER_TABLE_UUID: u32 = 0x2000_8080;
const TCODE_USER_RECORD: u32 = 0x2000_0081;

/// The RDK plug-in (`RdkPlugInId`): its user table is the RDK document.
const RDK_PLUG_IN: Uuid = Uuid::parse("16592D58-4A2F-401D-BF5E-3B87741C1B1B");
/// `ON_RdkUserData`: RDK data of objects (decals) and, in old files, of materials.
const ON_RDK_USER_DATA: Uuid = Uuid::parse("AFA82772-1525-43dd-A63C-C84AC5806911");
/// The user data classes of the mesh modifiers.
const ON_DISPLACEMENT_USER_DATA: Uuid = Uuid::parse("B8C04604-B4EF-43b7-8C26-1AFB8F1C54EB");
const ON_EDGE_SOFTENING_USER_DATA: Uuid = Uuid::parse("CB5EB395-BF1B-4112-8F2F-F728FCE8169C");
const ON_THICKENING_USER_DATA: Uuid = Uuid::parse("AA03D9C3-4CCF-4431-A06E-25F38CF3913F");
const ON_CURVE_PIPING_USER_DATA: Uuid = Uuid::parse("2D5AFEA9-F458-4079-992F-C2D405D9383B");
const ON_SHUT_LINING_USER_DATA: Uuid = Uuid::parse("429DCD06-5643-4254-BDE8-C0557F8FD083");

/// The largest XML document read (UTF-8 bytes).
const MAX_XML: usize = 64 << 20;
/// At most this many render contents, decals and shut lining curves are read.
const MAX_ITEMS: usize = 100_000;

/// `ON_3dmRenderSettings::Read`: the Rhino 6 and later chunk (from openNURBS 2013-11-05), or
/// the older versioned list of Rhino 2 to 5.
pub fn read_render_settings(mut c: Cur<'_>, s: &mut RenderSettings) -> R<()> {
    // the Rhino 6 chunk from openNURBS 6.0.2013-11-05 (ON_VersionNumberConstruct)
    let old = c.v.archive <= 50 || (c.v.archive <= 60 && c.v.opennurbs < 2_348_830_560);
    if old {
        let version = c.i32()?;
        if !(100..200).contains(&version) {
            return err("render settings of an unknown version");
        }
        s.custom_image_size = c.i32()? != 0;
        s.image_width = c.i32()?;
        s.image_height = c.i32()?;
        s.ambient = c.color()?;
        s.background_style = c.i32()?;
        s.background_color = c.color()?;
        s.wallpaper = c.wstring()?;
        s.use_hidden_lights = c.i32()? != 0;
        s.depth_cue = c.i32()? != 0;
        s.flat_shade = c.i32()? != 0;
        s.render_backfaces = c.i32()? != 0;
        s.render_points = c.i32()? != 0;
        s.render_curves = c.i32()? != 0;
        s.render_isoparams = c.i32()? != 0;
        s.render_mesh_edges = c.i32()? != 0;
        s.render_annotations = c.i32()? != 0;
        s.antialias = c.i32()?;
        s.shadowmap = c.i32()?;
        s.shadowmap_width = c.i32()?;
        s.shadowmap_height = c.i32()?;
        s.shadowmap_offset = c.f64()?;
        if version >= 101 {
            s.image_dpi = c.f64()?;
            s.image_unit = c.i32()?;
        }
        if version >= 102 {
            s.background_bottom_color = c.color()?;
        }
        if version >= 103 {
            s.scale_background_to_fit = c.bool()?;
        }
        return Ok(());
    }
    let (mut b, major, minor) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    if major != 1 {
        return err("render settings of an unknown version");
    }
    s.custom_image_size = b.bool()?;
    s.image_width = b.i32()?;
    s.image_height = b.i32()?;
    s.image_dpi = b.f64()?;
    s.image_unit = b.i32()?;
    s.ambient = b.color()?;
    s.background_style = b.i32()?;
    s.background_color = b.color()?;
    s.background_bottom_color = b.color()?;
    s.wallpaper = b.wstring()?;
    for flag in [
        &mut s.use_hidden_lights,
        &mut s.depth_cue,
        &mut s.flat_shade,
        &mut s.render_backfaces,
        &mut s.render_points,
        &mut s.render_curves,
        &mut s.render_isoparams,
        &mut s.render_mesh_edges,
        &mut s.render_annotations,
        &mut s.scale_background_to_fit,
        &mut s.transparent_background,
    ] {
        *flag = b.bool()?;
    }
    s.antialias = b.i32()?;
    s.shadowmap = b.i32()?;
    s.shadowmap_width = b.i32()?;
    s.shadowmap_height = b.i32()?;
    s.shadowmap_offset = b.f64()?;
    if minor >= 1 {
        b.take(4 + 8 * 3 + 4)?; // focal blur, now kept by the views
    }
    if minor >= 2 {
        s.rendering_source = b.i32()?;
        s.specific_viewport = b.wstring()?;
        s.named_view = b.wstring()?;
        s.snapshot = b.wstring()?;
    }
    if minor >= 3 {
        s.force_viewport_aspect_ratio = b.bool()?;
    }
    Ok(())
}

/// A user table (`TCODE_USER_TABLE`): the XML of the RDK document when it is the RDK plug-in's
/// (`GetEntireRDKDocument`: version 1 a UTF-16 string, versions 3 and 4 UTF-8, 4 followed by the
/// embedded files, which are not read).
pub fn read_rdk_document(mut c: Cur<'_>) -> R<Option<String>> {
    let id = c.chunk()?;
    if id.typecode != TCODE_USER_TABLE_UUID || id.body.clone().uuid()? != RDK_PLUG_IN {
        return Ok(None);
    }
    let record = c.chunk()?;
    if record.typecode != TCODE_USER_RECORD {
        return Ok(None);
    }
    let mut r = record.body;
    match r.i32()? {
        1 => Ok(Some(r.wstring()?)),
        3 | 4 => {
            let n = r.i32()?;
            let n = usize::try_from(n).ok().filter(|&n| n > 0 && n <= MAX_XML).ok_or("Rhino 3dm: an RDK document of a bad size")?;
            Ok(Some(String::from_utf8_lossy(trim_zero(r.take(n)?)).into_owned()))
        }
        _ => Ok(None),
    }
}

fn trim_zero(b: &[u8]) -> &[u8] {
    &b[..b.iter().position(|&x| x == 0).unwrap_or(b.len())]
}

/// `ON_XMLUserData::Read`: version 1 a UTF-16 string, else the length and the UTF-8 bytes.
fn xml_user_data(mut c: Cur<'_>) -> R<Node> {
    let text = match c.i32()? {
        1 => c.wstring()?,
        _ => {
            let n = usize::try_from(c.i32()?).ok().filter(|&n| n <= MAX_XML).ok_or("Rhino 3dm: XML user data of a bad size")?;
            String::from_utf8_lossy(trim_zero(c.take(n)?)).into_owned()
        }
    };
    xml::parse(&text)
}

/// The node of a document or of XML user data with this tag: the top-level one, or the one
/// below a top-level `xml` node.
fn top<'n>(root: &'n Node, tag: &str) -> Option<&'n Node> {
    root.child(tag).or_else(|| root.child("xml").and_then(|x| x.child(tag)))
}

/// The RDK document's rendering settings into `s`, and its render content.
pub fn read_document(text: &str, s: &mut RenderSettings) -> R<Vec<RenderContent>> {
    let root = xml::parse(text)?;
    let Some(doc) = top(&root, "render-content-manager-document") else { return Ok(Vec::new()) };
    let settings = doc.child("settings");
    let p = |path: &str| settings.and_then(|n| n.path(path));
    s.background_environment = uuid_param(doc.child("content"), "environment").unwrap_or_default();
    let rendering = p("rendering");
    s.reflection_environment_on = bool_param(rendering, "custom-env-for-refl-and-refr-on", false);
    s.reflection_environment = uuid_param(rendering, "custom-env-for-refl-and-refr").unwrap_or_default();
    s.linear_workflow.pre_process = bool_param(rendering, "use-linear-workflow", false);
    s.linear_workflow.post_process = bool_param(rendering, "use-post-process-gamma", true);
    s.linear_workflow.gamma = f64_param(rendering, "gamma", 2.2).clamp(0.2, 5.0);
    s.dithering.on = bool_param(rendering, "use-dithering", false);
    s.dithering.method = if text_param(rendering, "dithering") == "floyd-steinberg" { "floyd-steinberg" } else { "simple-noise" };
    let channels = p("rendering/render-channels");
    s.render_channels_custom = text_param(channels, "mode") == "custom";
    s.render_channels = text_param(channels, "list").split(';').filter_map(parse_uuid).collect();

    let g = p("ground-plane");
    let gp = &mut s.ground_plane;
    gp.on = bool_param(g, "on", false);
    gp.altitude = f64_param(g, "altitude", 0.0);
    gp.auto_altitude = bool_param(g, "auto-altitude", false);
    gp.shadow_only = bool_param(g, "shadow-only", true);
    gp.show_underside = bool_param(g, "show-underside", false);
    gp.material = uuid_param(g, "material").unwrap_or_default();
    gp.texture_offset = numbers::<2>(g, "texture-offset");
    gp.texture_size = numbers::<2>(g, "texture-size");
    gp.texture_rotation = f64_param(g, "texture-rotation", 0.0);
    gp.texture_offset_locked = bool_param(g, "offset-lock", false);
    gp.texture_size_locked = bool_param(g, "repeat-lock", false);

    let n = p("sun");
    let sun = &mut s.sun;
    sun.on = bool_param(n, "enable-on", false);
    sun.manual = bool_param(n, "manual-control-on", false);
    sun.north = f64_param(n, "north", 90.0);
    sun.latitude = f64_param(n, "observer-latitude", 0.0);
    sun.longitude = f64_param(n, "observer-longitude", 0.0);
    sun.time_zone = f64_param(n, "observer-timezone", 0.0);
    sun.daylight_saving_on = bool_param(n, "daylight-saving-on", false);
    sun.daylight_saving_minutes = i32_param(n, "daylight-saving-minutes", 0);
    sun.year = i32_param(n, "year", 2000);
    sun.month = i32_param(n, "month", 1);
    sun.day = i32_param(n, "day", 1);
    sun.hours = f64_param(n, "time", 12.0);
    sun.intensity = f64_param(n, "intensity", 1.0);
    sun.shadow_intensity = f64_param(n, "shadow-intensity", 1.0);
    if sun.manual {
        sun.azimuth = f64_param(n, "sun-azimuth", 0.0);
        sun.altitude = f64_param(n, "sun-altitude", 0.0);
    } else {
        let dst = if sun.daylight_saving_on { sun.daylight_saving_minutes } else { 0 };
        (sun.azimuth, sun.altitude) = sun_position(sun.latitude, sun.longitude, sun.time_zone, dst, sun.year, sun.month, sun.day, sun.hours);
    }
    sun.color = sun_color(sun.altitude);
    // ConvertHorizonCoordsToSolarVector of the azimuth turned to the model's north
    let azimuth = sun.azimuth + 90.0 - sun.north;
    let (a, h) = (azimuth.to_radians(), sun.altitude.to_radians());
    sun.direction = [-h.cos() * a.sin(), -h.cos() * a.cos(), -h.sin()];
    s.skylight.on = bool_param(n, "skylight-on", false);
    s.skylight.shadow_intensity = f64_param(n, "skylight-shadow-intensity", 1.0);
    s.skylight.environment_on = bool_param(n, "skylight-custom-environment-on", false);
    s.skylight.environment = uuid_param(n, "skylight-custom-environment").unwrap_or_default();

    let f = p("safe-frame");
    let sf = &mut s.safe_frame;
    sf.on = bool_param(f, "on", false);
    sf.perspective_only = bool_param(f, "perspective-only", false);
    sf.field_grid_on = bool_param(f, "field-display-on", false);
    sf.live_frame_on = bool_param(f.and_then(|f| f.child("live-frame")), "on", false);
    let frame = |name: &str, scale: f64| {
        let n = f.and_then(|f| f.child(name));
        SafeFrameFrame { on: bool_param(n, "on", false), linked: bool_param(n, "link", false), x_scale: f64_param(n, "x-scale", scale), y_scale: f64_param(n, "y-scale", scale) }
    };
    sf.action_frame = frame("action-frame", 0.9);
    sf.title_frame = frame("title-frame", 0.8);

    if let Some(peps) = p("post-effects-new") {
        for (stage, tag) in [("early", "early"), ("tone-mapping", "tone-mapping"), ("late", "late")] {
            for pep in peps.child(tag).map(|n| n.children.as_slice()).unwrap_or_default().iter().filter(|n| n.tag == "plug-in") {
                if s.post_effects.len() >= MAX_ITEMS {
                    break;
                }
                s.post_effects.push(PostEffect {
                    id: pep.attribute("id").and_then(parse_uuid).unwrap_or_default(),
                    name: pep.attribute("name").unwrap_or_default().to_string(),
                    stage,
                    on: pep.attribute("on").is_some_and(as_bool),
                    shown: pep.attribute("shown").is_some_and(as_bool),
                    parameters: pep.child("state").map(parameters).unwrap_or_default(),
                });
            }
        }
    }

    let mut contents = Vec::new();
    for (kind, section) in [("material", "material-section"), ("environment", "environment-section"), ("texture", "texture-section")] {
        for node in doc.child(section).map(|n| n.children.as_slice()).unwrap_or_default() {
            content(node, kind, Uuid::default(), &mut contents);
        }
    }
    Ok(contents)
}

/// The kinds of render content by tag.
fn content_kind(tag: &str) -> Option<&'static str> {
    match tag {
        "material" => Some("material"),
        "environment" => Some("environment"),
        "texture" => Some("texture"),
        _ => None,
    }
}

/// A render content node (`ON_RenderContent`: its properties as attributes, its parameters,
/// its simulation and the content in its child slots), then its children after it.
fn content(node: &Node, kind: &'static str, parent: Uuid, out: &mut Vec<RenderContent>) {
    if content_kind(&node.tag).is_none() || out.len() >= MAX_ITEMS {
        return;
    }
    let a = |name: &str| node.attribute(name).unwrap_or_default();
    let id = parse_uuid(a("instance-id")).unwrap_or_default();
    let params = node.child("parameters-v8").or_else(|| node.child("parameters"));
    let mut c = RenderContent {
        kind: content_kind(&node.tag).unwrap_or(kind),
        type_name: a("type-name").to_string(),
        type_id: parse_uuid(a("type-id")).unwrap_or_default(),
        id,
        name: a("instance-name").to_string(),
        render_engine: parse_uuid(a("render-engine-id")).unwrap_or_default(),
        plug_in: parse_uuid(a("plug-in-id")).unwrap_or_default(),
        group: parse_uuid(a("group-id")).unwrap_or_default(),
        notes: a("notes").to_string(),
        tags: a("tags").to_string(),
        hidden: as_bool(a("hidden")),
        reference: as_bool(a("reference")),
        auto_delete: as_bool(a("auto-delete")),
        child_slot: a("child-slot-name").to_string(),
        parent,
        children: Vec::new(),
        parameters: params.map(parameters).unwrap_or_default(),
        simulation: node.child("simulation").map(parameters).unwrap_or_default(),
    };
    let children: Vec<&Node> = node.children.iter().filter(|n| content_kind(&n.tag).is_some()).collect();
    c.children = children.iter().filter_map(|n| n.attribute("instance-id").and_then(parse_uuid)).collect();
    out.push(c);
    for child in children {
        content(child, kind, id, out);
    }
}

/// The parameters of a node: of the V8 layout (`<parameter name=".." type="..">`) or the older
/// one (a node per parameter, named by its tag), in order.
fn parameters(node: &Node) -> Vec<(String, RenderValue)> {
    node.children
        .iter()
        .take(MAX_ITEMS)
        .map(|n| {
            let name = if n.tag == "parameter" { n.attribute("name").unwrap_or_default() } else { n.tag.as_str() };
            (name.to_string(), value(n))
        })
        .collect()
}

/// A parameter's value by its `type` (`ON_XMLParameters::GetParamNode`).
fn value(n: &Node) -> RenderValue {
    let text = n.text.trim();
    match n.attribute("type").unwrap_or_default().to_ascii_lowercase().as_str() {
        "bool" => RenderValue::Bool(as_bool(text)),
        "int" | "short" | "char" | "long" | "integer" => RenderValue::Integer(as_int(text)),
        "float" | "double" | "real" => RenderValue::Number(as_f64(text)),
        "uuid" => parse_uuid(text).map_or_else(|| RenderValue::Text(text.to_string()), RenderValue::Uuid),
        "color" | "point" => {
            let v = as_numbers(text);
            let at = |i: usize, d: f64| v.get(i).copied().unwrap_or(d) as f32;
            RenderValue::Color([at(0, 0.0), at(1, 0.0), at(2, 0.0), at(3, 1.0)])
        }
        "vector2d" | "vector3d" | "vector" | "point4d" | "4da" | "matrix" | "matrix-4x4" => RenderValue::Numbers(as_numbers(text)),
        "" if text.is_empty() => RenderValue::Null,
        _ => RenderValue::Text(n.text.clone()),
    }
}

/// `ON_XMLVariant::AsBool` of a string: "true" or "t", else a non-zero number.
fn as_bool(s: &str) -> bool {
    let s = s.trim();
    s.eq_ignore_ascii_case("true") || s.eq_ignore_ascii_case("t") || as_int(s) != 0
}

/// `ON_wtoi`: the leading integer of a string (0 if none).
fn as_int(s: &str) -> i64 {
    let s = s.trim();
    let end = s.char_indices().find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+')))).map_or(s.len(), |(i, _)| i);
    s[..end].parse().unwrap_or(0)
}

fn as_f64(s: &str) -> f64 {
    s.trim().parse().unwrap_or(0.0)
}

/// Numbers separated by commas (points, vectors, colours, matrices).
fn as_numbers(s: &str) -> Vec<f64> {
    s.split(',').take(16).map(as_f64).collect()
}

/// A uuid in its text form (braces allowed), when it is one.
pub fn parse_uuid(s: &str) -> Option<Uuid> {
    let s = s.trim().trim_start_matches('{').trim_end_matches('}');
    let groups: Vec<&str> = s.split('-').collect();
    let lengths = [8, 4, 4, 4, 12];
    if groups.len() != 5 || groups.iter().zip(lengths).any(|(g, n)| g.len() != n || !g.bytes().all(|b| b.is_ascii_hexdigit())) {
        return None;
    }
    Some(Uuid::parse(s))
}

/// The text of a child parameter node of `node`.
fn param<'n>(node: Option<&'n Node>, name: &str) -> Option<&'n str> {
    node.and_then(|n| n.child(name)).map(|c| c.text.as_str())
}

fn text_param(node: Option<&Node>, name: &str) -> String {
    param(node, name).map(|s| s.trim().to_string()).unwrap_or_default()
}

fn bool_param(node: Option<&Node>, name: &str, default: bool) -> bool {
    param(node, name).map_or(default, as_bool)
}

fn f64_param(node: Option<&Node>, name: &str, default: f64) -> f64 {
    param(node, name).and_then(|s| s.trim().parse().ok()).unwrap_or(default)
}

fn i32_param(node: Option<&Node>, name: &str, default: i32) -> i32 {
    param(node, name).map_or(default, |s| as_int(s).clamp(i32::MIN as i64, i32::MAX as i64) as i32)
}

fn uuid_param(node: Option<&Node>, name: &str) -> Option<Uuid> {
    param(node, name).and_then(parse_uuid)
}

fn numbers<const N: usize>(node: Option<&Node>, name: &str) -> [f64; N] {
    let v = param(node, name).map(as_numbers).unwrap_or_default();
    std::array::from_fn(|i| v.get(i).copied().unwrap_or(0.0))
}

/// The sun's compass azimuth and altitude (degrees) at a place and local time
/// (`ON_SunEngine`, of minimum accuracy: Meeus' algorithm with a fixed obliquity).
#[allow(clippy::too_many_arguments, clippy::excessive_precision)]
fn sun_position(latitude: f64, longitude: f64, time_zone: f64, daylight_minutes: i32, year: i32, month: i32, day: i32, hours: f64) -> (f64, f64) {
    let (sin, cos) = (|d: f64| d.to_radians().sin(), |d: f64| d.to_radians().cos());
    let unwind = |d: f64| {
        let d = (d / 360.0).fract() * 360.0;
        if d < 0.0 { d + 360.0 } else { d }
    };
    // SetLocalDateTime: the local Julian day
    let (mut y, mut m) = (year, month);
    if m < 3 {
        m += 12;
        y -= 1;
    }
    let a = y / 100;
    let b = 2 - a + a / 4;
    let jd = (36525 * (y + 4716)) / 100 + (306 * (m + 1)) / 10 + day + b - 1524;
    let local = jd as f64 + hours / 24.0 - 0.5;
    let ut = local - (time_zone + daylight_minutes as f64 / 60.0) / 24.0;
    let d2000 = ut - 2_451_545.0;
    let t = d2000 / 36525.0;
    let longitude_sun = unwind(280.46646 + 36000.76983 * t + 0.0003032 * t * t);
    let apparent = longitude_sun - 0.00569;
    let (sin_obliquity, cos_obliquity) = (0.39778850739794974, 0.91747714052291862);
    let declination = (sin(apparent) * sin_obliquity).asin().to_degrees();
    let ascension = unwind((sin(apparent) * cos_obliquity).atan2(cos(apparent)).to_degrees());
    let sidereal = unwind(280.46061837 + 360.98564736629 * d2000 + 0.000387933 * t * t - t * t * t / 38_710_000.0);
    let hour_angle = sidereal + longitude - ascension;
    let azimuth = unwind(180.0 + sin(hour_angle).atan2(cos(hour_angle) * sin(latitude) - declination.to_radians().tan() * cos(latitude)).to_degrees());
    let altitude = (sin(latitude) * sin(declination) + cos(latitude) * cos(declination) * cos(hour_angle)).asin().to_degrees();
    (azimuth, altitude)
}

/// `ON_Sun::SunColorFromAltitude`: the colour of a black body at a temperature that rises with
/// the altitude, relative to the monitor's white, dark blue below the twilight zone.
fn sun_color(altitude: f64) -> [f32; 4] {
    let dark = [0.0, 0.0, 64.0 / 255.0, 1.0];
    if altitude < -3.0 {
        return dark;
    }
    let planck = |lambda: f64, temp: f64| lambda.powf(-5.0) / ((14388.0 / (lambda * temp)).exp() - 1.0);
    let rgb = |temp: f64| {
        let e = [planck(0.60, temp), planck(0.56, temp), planck(0.44, temp)];
        let scale = 1.0 / e[0].max(e[1]).max(e[2]);
        e.map(|v| v * scale)
    };
    let white = rgb(5000.0);
    let c = rgb((30.0 * altitude.max(0.0).powf(1.5) + 500.0).min(5300.0));
    let c = [c[0] / white[0], c[1] / white[1], c[2] / white[2]];
    let scale = 1.0 / c[0].max(c[1]).max(c[2]);
    let mut out = [(c[0] * scale).powf(0.15) as f32, (c[1] * scale).powf(0.15) as f32, (c[2] * scale).powf(0.15) as f32, 1.0];
    if altitude < 0.0 {
        let t = (-altitude / 3.0).clamp(0.0, 1.0) as f32;
        for (o, d) in out.iter_mut().zip(dark) {
            *o += (d - *o) * t;
        }
    }
    out
}

/// The decals and mesh modifiers of an object, from the user data of its attributes and of its
/// geometry.
pub fn object_render(userdata: &[(Uuid, Cur<'_>)], decals: &mut Vec<Decal>, modifiers: &mut MeshModifiers) {
    for (class, data) in userdata {
        let Ok(root) = xml_user_data(data.clone()) else { continue };
        let node = |tag: &str| top(&root, tag);
        match *class {
            ON_RDK_USER_DATA => {
                let Some(list) = node("render-content-manager-data").and_then(|n| n.child("decals")) else { continue };
                for d in list.children.iter().filter(|n| n.tag == "decal") {
                    if decals.len() < MAX_ITEMS {
                        decals.push(decal(d));
                    }
                }
            }
            ON_DISPLACEMENT_USER_DATA => {
                let Some(n) = node("new-displacement-object-data") else { continue };
                let n = Some(n);
                modifiers.displacement = Some(Displacement {
                    on: bool_param(n, "on", false),
                    texture: uuid_param(n, "texture").unwrap_or_default(),
                    mapping_channel: i32_param(n, "channel", 0),
                    black_point: f64_param(n, "black-point", 0.0),
                    white_point: f64_param(n, "white-point", 1.0),
                    sweep_pitch: i32_param(n, "sweep-pitch", 1000),
                    refine_steps: i32_param(n, "refine-steps", 1),
                    refine_sensitivity: f64_param(n, "refine-sensitivity", 0.5),
                    face_limit_on: bool_param(n, "face-count-limit-enabled", false),
                    face_limit: i32_param(n, "face-count-limit", 10000),
                    fairing_on: bool_param(n, "fairing-enabled", false),
                    fairing: i32_param(n, "fairing-amount", 4),
                    post_weld_angle: f64_param(n, "post-weld-angle", 40.0),
                    memory_limit: i32_param(n, "mesh-memory-limit", 512),
                    // files before Rhino 6 used the formula of the absolute tolerance
                    sweep_formula: i32_param(n, "sweep-res-formula", if data.v.archive < 60 { 1 } else { 0 }),
                });
            }
            ON_EDGE_SOFTENING_USER_DATA => {
                let Some(n) = node("edge-softening-object-data") else { continue };
                let n = Some(n);
                modifiers.edge_softening = Some(EdgeSoftening {
                    on: bool_param(n, "on", false),
                    softening: f64_param(n, "softening", 0.1),
                    chamfer: bool_param(n, "chamfer", false),
                    faceted: bool_param(n, "unweld", false),
                    edge_angle_threshold: f64_param(n, "edge-threshold", 5.0),
                    force_softening: bool_param(n, "force-softening", false),
                });
            }
            ON_THICKENING_USER_DATA => {
                let Some(n) = node("thickening-object-data") else { continue };
                let n = Some(n);
                modifiers.thickening = Some(Thickening {
                    on: bool_param(n, "on", false),
                    distance: f64_param(n, "distance", 0.1),
                    solid: bool_param(n, "solid", true),
                    offset_only: bool_param(n, "offset-only", false),
                    both_sides: bool_param(n, "both-sides", false),
                });
            }
            ON_CURVE_PIPING_USER_DATA => {
                let Some(n) = node("curve-piping-object-data") else { continue };
                let n = Some(n);
                modifiers.curve_piping = Some(CurvePiping {
                    on: bool_param(n, "on", false),
                    radius: f64_param(n, "radius", 1.0),
                    segments: i32_param(n, "segments", 16),
                    // "weld" is the opposite of faceted
                    faceted: !bool_param(n, "weld", true),
                    accuracy: i32_param(n, "accuracy", 50),
                    cap_type: match text_param(n, "cap-type").as_str() {
                        "flat" => "flat",
                        "box" => "box",
                        "dome" => "dome",
                        _ => "none",
                    },
                });
            }
            ON_SHUT_LINING_USER_DATA => {
                let Some(node) = node("shut-lining-object-data") else { continue };
                let n = Some(node);
                let curves = node.children.iter().filter(|c| c.tag == "curve").take(MAX_ITEMS).map(|c| {
                    let c = Some(c);
                    ShutLiningCurve {
                        id: uuid_param(c, "uuid").unwrap_or_default(),
                        enabled: bool_param(c, "enabled", true),
                        radius: f64_param(c, "radius", 0.1),
                        profile: i32_param(c, "profile", 1),
                        pull: bool_param(c, "pull", true),
                        is_bump: bool_param(c, "is-bump", false),
                    }
                });
                modifiers.shut_lining = Some(ShutLining {
                    on: bool_param(n, "on", false),
                    faceted: bool_param(n, "faceted", false),
                    auto_update: bool_param(n, "auto-update", false),
                    force_update: bool_param(n, "force-update", false),
                    curves: curves.collect(),
                });
            }
            _ => {}
        }
    }
}

/// `ON_Decal`, from its node of the object's RDK data.
fn decal(d: &Node) -> Decal {
    let n = Some(d);
    let point = |name: &str| numbers::<3>(n, name);
    Decal {
        texture: uuid_param(n, "texture-instance").unwrap_or_default(),
        mapping: param(n, "mapping").map_or("uv", str::trim).to_string(),
        projection: param(n, "projection").map_or("none", str::trim).to_string(),
        map_to_inside: bool_param(n, "map-to-inside-on", false),
        transparency: f64_param(n, "transparency", 0.0),
        visible: bool_param(n, "is-visible", true),
        origin: point("origin"),
        vector_up: point("vector-up"),
        vector_across: point("vector-across"),
        height: f64_param(n, "height", 1.0),
        radius: f64_param(n, "radius", 1.0),
        horizontal_sweep: [f64_param(n, "latitude-start", 0.0), f64_param(n, "latitude-stop", 0.0)],
        vertical_sweep: [f64_param(n, "longitude-start", 0.0), f64_param(n, "longitude-stop", 0.0)],
        uv_bounds: [f64_param(n, "min-u", 0.0), f64_param(n, "min-v", 0.0), f64_param(n, "max-u", 1.0), f64_param(n, "max-v", 1.0)],
    }
}

/// The render material of a material kept in the obsolete RDK user data of files before Rhino 6
/// (`ON_RdkMaterialInstanceIdObsoleteUserData`: `<material instance-id="..">`).
pub fn obsolete_material_instance(userdata: &[(Uuid, Cur<'_>)]) -> Option<Uuid> {
    let (_, data) = userdata.iter().find(|(class, _)| *class == ON_RDK_USER_DATA)?;
    let root = xml_user_data(data.clone()).ok()?;
    top(&root, "render-content-manager-data")?.child("material")?.attribute("instance-id").and_then(parse_uuid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sun_is_where_rhino_puts_it() {
        // Delft, 21 June 2026 15:30 summer time, as Rhino 9 places it (render.json)
        let (azimuth, altitude) = sun_position(52.01, 4.36, 1.0, 60, 2026, 6, 21, 15.5);
        assert!((azimuth - 226.03560039754322).abs() < 1e-9, "{azimuth}");
        assert!((altitude - 54.806412638442474).abs() < 1e-9, "{altitude}");
        let c = sun_color(altitude);
        assert_eq!(c.map(|v| (v * 255.0).round() as u8), [251, 252, 255, 255]);
    }

    #[test]
    fn post_effects_and_contents_of_a_document() {
        let text = r#"<xml><render-content-manager-document><settings><post-effects-new><early/><tone-mapping>
            <plug-in name="Filmic" id="ec302d96-402e-4244-bf25-5d0859ebf37c" on="true" shown="false"><state><gamma type="double">2.2</gamma></state></plug-in>
            </tone-mapping><late/></post-effects-new><sun><manual-control-on type="bool">true</manual-control-on><sun-azimuth type="double">90</sun-azimuth>
            <sun-altitude type="double">45</sun-altitude></sun></settings><material-section><material type-name="rcm-basic-material" instance-id="7D90CE29-D774-4A62-A2A7-B4D716F9572A" instance-name="Old">
            <parameters><diffuse type="color">1,0.5,0.25</diffuse></parameters>
            <texture instance-id="2F457042-F741-41B9-B7D6-D7627E27A658" child-slot-name="bitmap-texture"/></material></material-section></render-content-manager-document></xml>"#;
        let mut s = RenderSettings::default();
        let contents = read_document(text, &mut s).unwrap();
        assert_eq!(s.post_effects.len(), 1);
        let p = &s.post_effects[0];
        assert_eq!((p.name.as_str(), p.stage, p.on, p.shown), ("Filmic", "tone-mapping", true, false));
        assert_eq!(p.parameters, vec![("gamma".to_string(), RenderValue::Number(2.2))]);
        // by hand: the stored azimuth and altitude; the light comes from the east (north is +y)
        assert_eq!((s.sun.azimuth, s.sun.altitude), (90.0, 45.0));
        assert!((s.sun.direction[0] + 0.5f64.sqrt()).abs() < 1e-12 && s.sun.direction[1].abs() < 1e-12);
        assert_eq!(contents.len(), 2);
        assert_eq!(contents[0].parameter("diffuse"), Some(&RenderValue::Color([1.0, 0.5, 0.25, 1.0])));
        assert_eq!((contents[1].kind, contents[1].parent, contents[1].child_slot.as_str()), ("texture", contents[0].id, "bitmap-texture"));
        assert_eq!(contents[0].children, vec![contents[1].id]);
    }

    #[test]
    fn values_by_their_type() {
        let doc = xml::parse("<p><a type=\"bool\">true</a><b type=\"int\">12</b><c type=\"color\">0.5,0.25,1</c><d type=\"uuid\">6dc8463c-4ca2-463a-a40b-0598a645ce24</d><e type=\"vector2d\">1,2</e><f/></p>").unwrap();
        let v: Vec<_> = parameters(doc.child("p").unwrap()).into_iter().map(|(_, v)| v).collect();
        assert_eq!(v[0], RenderValue::Bool(true));
        assert_eq!(v[1], RenderValue::Integer(12));
        assert_eq!(v[2], RenderValue::Color([0.5, 0.25, 1.0, 1.0]));
        assert_eq!(v[3], RenderValue::Uuid(Uuid::parse("6dc8463c-4ca2-463a-a40b-0598a645ce24")));
        assert_eq!(v[4], RenderValue::Numbers(vec![1.0, 2.0]));
        assert_eq!(v[5], RenderValue::Null);
        assert_eq!(parse_uuid("not-a-uuid"), None);
        assert_eq!(parse_uuid("{6dc8463c-4ca2-463a-a40b-0598a645ce24}").map(|u| u.to_string()), Some("6dc8463c-4ca2-463a-a40b-0598a645ce24".to_string()));
    }
}
