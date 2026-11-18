//! Semantic readers of the 3dm reader: a Rhino file (Rhino 2 to 9, archive versions 2, 3, 4, 5,
//! 50, 60, 70, 80 and 90) to the neutral model of `threedm-model`. Written from McNeel's
//! openNURBS source, which McNeel publishes as the reference for reading and writing 3dm files:
//! the properties, settings, material, layer, block definition, linetype, section style,
//! dimension style, hatch pattern and object tables, and of each object its class data, its
//! attributes and its user data. Nothing of openNURBS is compiled in.
//!
//! Objects become meshes, polylines, points or block instances: meshes as stored; the render
//! meshes Rhino keeps with polysurfaces (Breps), surfaces and extrusions (the analysis meshes
//! when there are none), and when a file has none (saved by a script or another program, or with
//! "save small") a mesh made from the exact geometry as Rhino does on opening it
//! ([`tessellate`]), Rhino 2's own polysurface layout included; SubD objects by their limit
//! surface ([`subd`]); curves as polylines within Rhino's display tolerance; texts set in a font
//! at their style's height and model space scale, and hatches filled or in their pattern's lines
//! ([`drafting`]); point clouds and points; block instances with their definition and transform.
//! Meshes are read up to [`MAX_VERTICES`] vertices, so that a damaged or hostile file cannot
//! exhaust memory. Clipping planes are read with their rectangle, plane and the viewports they
//! clip, and the viewports of the file's views with them; lights with their kind, place, colour
//! and attenuation ([`Light`]). Layouts are among the views, with their paper ([`Page`]); their
//! details are read with their boundary, the model view they show and its scale. Objects the
//! reader does not know are left out with the reason.

use glam::DVec3;
use lights::{MAX_LIGHTS, TCODE_LIGHT_RECORD, TCODE_LIGHT_TABLE};
use threedm_core::*;
pub use threedm_model::{Attributes, Bitmap, BlockDefinition, ClipSelection, ClippingPlane, File, Geometry, Layer as RhinoLayer, Light, LightKind, LineCap, LineJoin, Linetype, Material, Mesh, Object as RhinoObject, Page, Pbr, SectionStyle, Stroke, Texture, View};

mod dimensions;
mod document;
pub mod drafting;
mod fonts;
mod lights;
mod styles;
mod settings;
mod render;
pub mod subd;
pub mod tessellate;
mod v1;
mod xml;

pub use document::Document;
pub use threedm_model::{ConstructionPlane, DimensionStyle, EarthAnchor, Font, GridDefaults, HistoryData, HistoryRecord, LayerState, MeshSettings, PlugIn, DocumentRenderSettings, Settings, Snapshot, UserTable};
pub use threedm_model::{
    CurvePiping, Decal, Displacement, Dithering, EdgeSoftening, GroundPlane, LinearWorkflow, MeshModifiers, PostEffect, RenderContent, RenderSettings,
    RenderValue, SafeFrame, SafeFrameFrame, ShutLining, ShutLiningCurve, Skylight, Sun, Thickening,
};
pub use threedm_core::{Uuid, Versions, R};

const MAGIC: &[u8] = b"3D Geometry File Format ";

/// Reads a Rhino file with the built-in font (feature `font`) to set texts in.
pub fn read(data: &[u8]) -> R<File<'_>> {
    read_with(data, &Options::default())
}

/// How a file is read.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// The TrueType or OpenType font texts are set in (by their capital height); without one,
    /// texts are left out.
    pub font: Option<&'static [u8]>,
    /// The font file of a font the file names (family, bold, italic), when the caller has it:
    /// each text, dimension and leader is then set in its own font, else in `font`.
    pub resolve_font: Option<fn(&threedm_model::Font) -> Option<&'static [u8]>>,
}

impl Default for Options {
    fn default() -> Self {
        Options { font: if cfg!(feature = "font") { Some(FONT) } else { None }, resolve_font: None }
    }
}

#[cfg(feature = "font")]
const FONT: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
#[cfg(not(feature = "font"))]
const FONT: &[u8] = &[];

/// The file being read, what its objects are drawn with, and the mesh vertices read so far.
struct Reader<'a> {
    file: File<'a>,
    drafting: Drafting,
    /// Mesh vertices read so far, at most [`MAX_VERTICES`].
    vertices: usize,
}

/// What texts and hatches are drawn with.
struct Drafting {
    patterns: Vec<drafting::HatchPattern>,
    dim_styles: Vec<drafting::DimStyle>,
    /// The font the object being read is set in: its own when the caller resolves it, else
    /// `default_font`.
    font: std::cell::Cell<Option<&'static [u8]>>,
    default_font: Option<&'static [u8]>,
    resolve_font: Option<fn(&threedm_model::Font) -> Option<&'static [u8]>>,
    /// Metres per model unit, for lengths shown in other units.
    meters: f64,
    /// The document's annotation settings (Rhino 2 and 3), for annotations without a style.
    document_style: Option<drafting::DimStyle>,
    /// The font table of Rhino 4 and 5, by index.
    fonts: Vec<(i32, threedm_model::Font)>,
}

/// Reads a Rhino file: its tables, and its objects as meshes, polylines, points and block
/// instances.
pub fn read_with<'a>(data: &'a [u8], options: &Options) -> R<File<'a>> {
    // The header may follow some bytes of other data (OLE wrappers of old files).
    let start = data.windows(MAGIC.len()).take(1 << 20).position(|w| w == MAGIC).ok_or("Rhino 3dm: not a Rhino file (no 3dm header)")?;
    let header = data.get(start..start + 32).ok_or("Rhino 3dm: the file is too short")?;
    let version: u32 = std::str::from_utf8(&header[24..32]).ok().and_then(|s| s.trim().parse().ok()).ok_or("Rhino 3dm: the header has no version")?;
    let file = File { versions: Versions { archive: version, opennurbs: 0 }, meters: 0.001, unit: "millimetres", ..File::default() };
    let mut top = Cur { data, pos: start + 32, v: file.versions };
    let mut r = Reader { file, drafting: Drafting { patterns: Vec::new(), dim_styles: Vec::new(), font: std::cell::Cell::new(options.font), default_font: options.font, resolve_font: options.resolve_font, meters: 0.001, document_style: None, fonts: Vec::new() }, vertices: 0 };
    if version < 2 {
        // Rhino 1: top-level chunks of its own
        v1::read(top, &mut r.file, &mut r.vertices)?;
        return Ok(r.file);
    }
    // the RDK document (render content and settings), read once the settings are
    let mut rdk = None;
    while top.left() > 0 {
        let chunk = match top.chunk() {
            Ok(c) => c,
            // a damaged tail: keep what was read
            Err(e) if !r.file.objects.is_empty() => {
                *r.file.damaged.entry("end of file").or_default() += 1;
                let _ = e;
                break;
            }
            Err(e) => return Err(e),
        };
        let mut body = chunk.body;
        body.v = r.file.versions;
        match chunk.typecode {
            TCODE_ENDOFFILE => break,
            render::TCODE_USER_TABLE => {
                // the RDK document (rendering), and every plug-in's table as stored
                match render::read_rdk_document(body.clone()) {
                    Ok(Some(xml)) => rdk = Some(xml),
                    Ok(None) => {}
                    Err(_) => *r.file.damaged.entry("render content").or_default() += 1,
                }
                match settings::read_user_table(body) {
                    Ok(t) => {
                        settings::read_known_table(&t, r.file.versions, &mut r.file);
                        r.file.user_tables.push(t);
                    }
                    Err(_) => *r.file.damaged.entry("user tables").or_default() += 1,
                }
            }
            TCODE_PROPERTIES_TABLE => read_properties(body, &mut r.file),
            TCODE_SETTINGS_TABLE => read_settings(body, &mut r.file, &mut r.drafting),
            TCODE_MATERIAL_TABLE => table(body, TCODE_MATERIAL_RECORD, &mut r, "materials", |mut c, f| {
                let index = f.file.materials.len() as i32;
                if let Some(object) = c.object()?.filter(|o| o.class == ON_MATERIAL) {
                    f.file.materials.push(read_material(object, index)?);
                }
                Ok(())
            }),
            TCODE_BITMAP_TABLE => table(body, TCODE_BITMAP_RECORD, &mut r, "bitmaps", |mut c, f| {
                if let Some(object) = c.object()? {
                    f.file.bitmaps.extend(read_bitmap(object)?);
                }
                Ok(())
            }),
            TCODE_LAYER_TABLE => table(body, TCODE_LAYER_RECORD, &mut r, "layers", |mut c, f| {
                let index = f.file.layers.len() as i32;
                if let Some(object) = c.object()?.filter(|o| o.class == ON_LAYER) {
                    f.file.layers.push(read_layer(object.data, index)?);
                }
                Ok(())
            }),
            TCODE_INSTANCE_DEFINITION_TABLE => table(body, TCODE_INSTANCE_DEFINITION_RECORD, &mut r, "block definitions", |mut c, f| {
                if let Some(object) = c.object()?.filter(|o| o.class == ON_INSTANCE_DEFINITION) {
                    f.file.blocks.push(read_block_definition(object.data)?);
                }
                Ok(())
            }),
            TCODE_GROUP_TABLE => table(body, TCODE_GROUP_RECORD, &mut r, "groups", |mut c, f| {
                if let Some(object) = c.object()?.filter(|o| o.class == ON_GROUP) {
                    f.file.groups.push(read_group(object.data)?);
                }
                Ok(())
            }),
            styles::TCODE_LINETYPE_TABLE => table(body, styles::TCODE_LINETYPE_RECORD, &mut r, "linetypes", |mut c, f| {
                let position = f.file.linetypes.len() as i32;
                if let Some(object) = c.object()?.filter(|o| o.class == styles::ON_LINETYPE) {
                    f.file.linetypes.push(styles::read_linetype(object.data, position)?);
                }
                Ok(())
            }),
            styles::TCODE_SECTION_STYLE_TABLE => table(body, styles::TCODE_SECTION_STYLE_RECORD, &mut r, "section styles", |mut c, f| {
                let position = f.file.section_styles.len() as i32;
                if let Some(mut object) = c.object()?.filter(|o| o.class == styles::ON_SECTION_STYLE) {
                    f.file.section_styles.push(styles::read_section_style(&mut object.data, position)?);
                }
                Ok(())
            }),
            TCODE_DIMSTYLE_TABLE => table(body, TCODE_DIMSTYLE_RECORD, &mut r, "dimension styles", |mut c, f| {
                if let Some(object) = c.object()?.filter(|o| o.class == drafting::ON_DIM_STYLE || o.class == drafting::ON_V5X_DIM_STYLE) {
                    f.drafting.dim_styles.extend(drafting::read_dim_style(object.data)?);
                }
                Ok(())
            }),
            TCODE_HATCHPATTERN_TABLE => table(body, TCODE_HATCHPATTERN_RECORD, &mut r, "hatch patterns", |mut c, f| {
                let position = f.drafting.patterns.len() as i32;
                if let Some(object) = c.object()?.filter(|o| o.class == drafting::ON_HATCH_PATTERN) {
                    f.drafting.patterns.push(drafting::read_pattern(object.data, position)?);
                }
                Ok(())
            }),
            fonts::TCODE_FONT_TABLE => table(body, fonts::TCODE_FONT_RECORD, &mut r, "fonts", |mut c, f| {
                if let Some(object) = c.object()?.filter(|o| o.class == fonts::ON_TEXT_STYLE) {
                    f.drafting.fonts.push(fonts::read_v5_font(object.data)?);
                }
                Ok(())
            }),
            settings::TCODE_HISTORYRECORD_TABLE => table(body, settings::TCODE_HISTORYRECORD_RECORD, &mut r, "history records", |mut c, f| {
                if let Some(object) = c.object()?.filter(|o| o.class == settings::ON_HISTORY_RECORD) {
                    f.file.history.push(settings::read_history_record(object.data)?);
                }
                Ok(())
            }),
            TCODE_LIGHT_TABLE => table(body, TCODE_LIGHT_RECORD, &mut r, "lights", |c, f| {
                if f.file.lights.len() < MAX_LIGHTS {
                    f.file.lights.extend(lights::read_light_record(c)?);
                }
                Ok(())
            }),
            TCODE_OBJECT_TABLE => table(body, TCODE_OBJECT_RECORD, &mut r, "objects", |c, f| {
                if let Some(light) = lights::read_object_light(c.clone())? {
                    if f.file.lights.len() < MAX_LIGHTS {
                        f.file.lights.push(light);
                    }
                    return Ok(());
                }
                f.drafting.meters = f.file.meters;
                let mut object = read_object(c, &mut f.vertices, &f.drafting)?;
                if matches!(object.geometry, Geometry::Lines(_, "Curve")) {
                    object.stroke = Some(styles::stroke(&object.attributes, &f.file.layers, &f.file.linetypes, f.file.meters));
                }
                dash(&mut object, &f.file.layers, &f.file.linetypes);
                f.file.objects.push(object);
                Ok(())
            }),
            _ => {}
        }
    }
    if let Some(xml) = rdk {
        match render::read_document(&xml, &mut r.file.render) {
            Ok(contents) => r.file.render_content = contents,
            Err(_) => *r.file.damaged.entry("render content").or_default() += 1,
        }
    }
    finish(&mut r);
    Ok(r.file)
}

/// What is known once all is read: the dimension styles with their fonts, the font table, the
/// names of the plug-ins that saved tables.
fn finish(r: &mut Reader<'_>) {
    let fonts = &r.drafting.fonts;
    r.file.dim_styles = r
        .drafting
        .dim_styles
        .iter()
        .map(|d| threedm_model::DimensionStyle { id: d.id, index: d.index, name: d.name.clone(), font: d.font.clone().or_else(|| fonts.iter().find(|(i, _)| *i == d.text_style).map(|(_, f)| f.clone())) })
        .collect();
    r.file.fonts = fonts.clone();
    for t in &mut r.file.user_tables {
        if let Some(p) = r.file.settings.plugins.iter().find(|p| p.id == t.plugin) {
            t.name = p.name.clone();
        }
    }
}

/// The records of a table, up to its end; a damaged record is counted and skipped.
fn table<'a>(mut c: Cur<'a>, record: u32, r: &mut Reader<'a>, name: &'static str, mut each: impl FnMut(Cur<'a>, &mut Reader<'a>) -> R<()>) {
    while c.left() > 0 {
        let Ok(chunk) = c.chunk() else {
            *r.file.damaged.entry(name).or_default() += 1;
            break;
        };
        if chunk.typecode == TCODE_ENDOFTABLE {
            break;
        }
        if chunk.typecode == record && each(chunk.body, r).is_err() {
            *r.file.damaged.entry(name).or_default() += 1;
        }
    }
}

fn read_properties(mut c: Cur<'_>, file: &mut File<'_>) {
    while let Ok(chunk) = c.chunk() {
        match chunk.typecode {
            TCODE_ENDOFTABLE => break,
            TCODE_PROPERTIES_OPENNURBS_VERSION => file.versions.opennurbs = chunk.value as u32,
            TCODE_PROPERTIES_APPLICATION => {
                let mut b = chunk.body;
                if b.version().is_ok() {
                    file.application = b.wstring().unwrap_or_default();
                }
            }
            _ => {}
        }
    }
}

/// `TCODE_SETTINGS_ANNOTATION`: the annotation settings of the document (Rhino 2 and 3).
const TCODE_SETTINGS_ANNOTATION: u32 = 0x2000_8034;
/// `TCODE_SETTINGS_VIEW_LIST`: the views of the model's viewports, the active one first.
const TCODE_SETTINGS_VIEW_LIST: u32 = 0x2000_8037;
/// `TCODE_SETTINGS_ATTRIBUTES`: the document's settings attributes; from 1.1 the units of the
/// layouts.
const TCODE_SETTINGS_ATTRIBUTES: u32 = 0x2000_8134;
/// `TCODE_SETTINGS_NAMED_VIEW_LIST`: the views saved by name, as the view list.
const TCODE_SETTINGS_NAMED_VIEW_LIST: u32 = 0x2000_8036;
const TCODE_VIEW_RECORD: u32 = 0x2000_803B;
const TCODE_VIEW_VIEWPORT: u32 = 0x2000_823B;
const TCODE_VIEW_NAME: u32 = 0x2000_8A3B;
const TCODE_VIEW_ATTRIBUTES: u32 = 0x2000_8C3B;
/// At most this many views are read.
const MAX_VIEWS: usize = 1000;

/// The settings table: the units, and the document's annotation settings as a dimension style
/// for annotations that name none (Rhino 2).
fn read_settings(mut c: Cur<'_>, file: &mut File<'_>, drafting: &mut Drafting) {
    while let Ok(chunk) = c.chunk() {
        match chunk.typecode {
            TCODE_ENDOFTABLE => break,
            TCODE_SETTINGS_ANNOTATION => {
                if let Ok(style) = drafting::read_annotation_settings(chunk.body) {
                    drafting.document_style = Some(style);
                }
            }
            TCODE_SETTINGS_UNITSANDTOLS => {
                if let Ok(Some((meters, unit))) = read_units(chunk.body) {
                    file.meters = meters;
                    file.unit = unit;
                    // the page's units are the model's until the settings attributes say
                    if file.page_unit.is_empty() {
                        (file.page_meters, file.page_unit) = (meters, unit);
                    }
                }
            }
            TCODE_SETTINGS_ATTRIBUTES => {
                if let Ok(Some((meters, unit))) = read_page_units(chunk.body.clone()) {
                    (file.page_meters, file.page_unit) = (meters, unit);
                }
                settings::read_chunk(&chunk, &mut file.settings);
            }
            TCODE_SETTINGS_VIEW_LIST => {
                if let Ok(views) = read_views(chunk.body.clone()) {
                    file.views = views;
                }
                settings::read_chunk(&chunk, &mut file.settings);
            }
            TCODE_SETTINGS_NAMED_VIEW_LIST => {
                if let Ok(views) = read_views(chunk.body) {
                    file.named_views = views;
                }
            }
            render::TCODE_SETTINGS_RENDER => {
                if render::read_render_settings(chunk.body.clone(), &mut file.render).is_err() {
                    *file.damaged.entry("render settings").or_default() += 1;
                }
                settings::read_chunk(&chunk, &mut file.settings);
            }
            _ => settings::read_chunk(&chunk, &mut file.settings),
        }
    }
}

/// `TCODE_SETTINGS_VIEW_LIST` (and the named view list): a count and as many `ON_3dmView`
/// records.
fn read_views(mut c: Cur<'_>) -> R<Vec<View>> {
    let count = c.i32()?;
    if !(0..=MAX_VIEWS as i32).contains(&count) {
        return err("a view list of an unknown length");
    }
    let mut views = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let record = c.chunk()?;
        if record.typecode != TCODE_VIEW_RECORD {
            return err("a view record of an unknown kind");
        }
        views.extend(read_view(record.body)?.map(|(view, _)| view));
    }
    Ok(views)
}

/// `ON_3dmView::Read`: chunks up to `TCODE_ENDOFTABLE`; of them its name, from its `ON_Viewport`
/// the viewport's id, projection and camera, and from its attributes whether it is a layout
/// and its paper, and whether its projection is locked. `None` without a viewport.
fn read_view(mut c: Cur<'_>) -> R<Option<(View, bool)>> {
    let (mut name, mut viewport, mut page, mut locked) = (String::new(), None, None, false);
    while let Ok(chunk) = c.chunk() {
        match chunk.typecode {
            TCODE_ENDOFTABLE => break,
            TCODE_VIEW_NAME => name = chunk.body.clone().wstring().unwrap_or_default(),
            TCODE_VIEW_VIEWPORT => viewport = read_viewport(chunk.body).ok(),
            TCODE_VIEW_ATTRIBUTES => {
                if let Ok((p, l)) = read_view_attributes(chunk.body) {
                    (page, locked) = (p, l);
                }
            }
            _ => {}
        }
    }
    Ok(viewport.map(|mut view: View| {
        view.name = name;
        view.page = page;
        (view, locked)
    }))
}

/// `TCODE_VIEW_ATTRIBUTES` (1.1 and later, from openNURBS 2005-03-17): the view type (1 a
/// page), the paper's width and height (mm), an obsolete parent id and box; from 1.2 the
/// display mode and `ON_3dmPageSettings` (its page number, then the paper again); from 1.3
/// whether the projection is locked.
fn read_view_attributes(mut c: Cur<'_>) -> R<(Option<Page>, bool)> {
    let (major, minor) = c.version()?;
    if major != 1 || minor < 1 || c.v.archive < 4 || c.v.opennurbs < 200_503_170 {
        return Ok((None, false));
    }
    let view_type = c.i32()?;
    let mut page = Page { width: c.f64()?, height: c.f64()?, number: 0 };
    c.take(16 + 48)?; // obsolete parent viewport id and nested view position
    let mut locked = false;
    if minor >= 2 {
        c.uuid()?; // display mode
        let (mut settings, major, _) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
        if major == 1 {
            page = Page { number: settings.i32()?, width: settings.f64()?, height: settings.f64()? };
        }
        if minor >= 3 {
            locked = c.bool()?;
        }
    }
    Ok(((view_type == 1).then_some(page), locked))
}

/// `ON_Viewport::Read` up to its id: the projection (2 is perspective), the camera (location,
/// direction, up, its frame), the frustum (left, right, bottom, top, near, far), the port and,
/// from version 1.1, the viewport id.
fn read_viewport(mut c: Cur<'_>) -> R<View> {
    let (major, minor) = c.version()?;
    if major != 1 {
        return err("a viewport of an unknown version");
    }
    c.take(12)?; // valid camera, frustum, port
    let projection = c.i32()?;
    let mut point = || -> R<DVec3> { Ok(DVec3::new(c.f64()?, c.f64()?, c.f64()?)) };
    let (location, direction, up) = (point()?, point()?, point()?);
    c.take(3 * 24)?; // camera frame
    let f: Vec<f64> = (0..6).map(|_| c.f64()).collect::<R<_>>()?;
    c.take(6 * 4)?; // port
    let id = if minor >= 1 { c.uuid()? } else { Uuid::default() };
    let (bottom, top, near) = (f[2], f[3], f[4]);
    let perspective = projection == 2;
    let fov_y = if perspective && near > 0.0 { 2.0 * (0.5 * (top - bottom) / near).atan() } else { 0.0 };
    Ok(View { id, name: String::new(), perspective, location, direction: direction.normalize_or_zero(), up: up.normalize_or_zero(), fov_y, height: top - bottom, page: None })
}

/// The page units of `ON_3dmSettings`' attributes (version 1.1 and later): after the linetype
/// display scale, the current plot colour and its source, the current linetype and its
/// source, a chunk with `ON_3dmUnitsAndTolerances`.
fn read_page_units(mut c: Cur<'_>) -> R<Option<(f64, &'static str)>> {
    let (major, minor) = c.version()?;
    if major != 1 || minor < 1 {
        return Ok(None);
    }
    c.take(8 + 4 + 4 + 4 + 4)?;
    let (units, _, _) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    read_units(units)
}

/// `ON_3dmUnitsAndTolerances`: the length unit, as metres per unit and its name.
fn read_units(mut c: Cur<'_>) -> R<Option<(f64, &'static str)>> {
    let version = c.i32()?;
    if !(100..200).contains(&version) {
        return Ok(None);
    }
    let unit = c.u32()?;
    c.take(24)?; // absolute, angle and relative tolerance
    let mut custom = None;
    if version >= 102 {
        c.take(8)?; // distance display mode and precision
        custom = Some(c.f64()?);
    }
    Ok(unit_scale(unit, custom))
}

/// Metres per unit of `ON::LengthUnitSystem`.
fn unit_scale(unit: u32, custom: Option<f64>) -> Option<(f64, &'static str)> {
    Some(match unit {
        0 => (1.0, "none (taken as metres)"),
        1 => (1e-6, "microns"),
        2 => (1e-3, "millimetres"),
        3 => (1e-2, "centimetres"),
        4 => (1.0, "metres"),
        5 => (1e3, "kilometres"),
        6 => (2.54e-8, "microinches"),
        7 => (2.54e-5, "mils"),
        8 => (0.0254, "inches"),
        9 => (0.3048, "feet"),
        10 => (1609.344, "miles"),
        12 => (1e-10, "ångströms"),
        13 => (1e-9, "nanometres"),
        14 => (0.1, "decimetres"),
        15 => (10.0, "dekametres"),
        16 => (100.0, "hectometres"),
        17 => (1e6, "megametres"),
        18 => (1e9, "gigametres"),
        19 => (0.9144, "yards"),
        20 => (0.0254 / 72.0, "printer points"),
        21 => (0.0254 / 6.0, "printer picas"),
        22 => (1852.0, "nautical miles"),
        23 => (1.495_978_707e11, "astronomical units"),
        24 => (9.460_730_472_580_8e15, "light years"),
        25 => (3.085_677_581_49e16, "parsecs"),
        11 => (custom.filter(|m| m.is_finite() && *m > 0.0)?, "custom units"),
        _ => return None,
    })
}

/// `ON_Layer` (chunk version 1.x, all Rhino versions).
fn read_layer(mut c: Cur<'_>, index: i32) -> R<RhinoLayer> {
    let (major, minor) = c.version()?;
    if major != 1 {
        return err("a layer of an unknown version");
    }
    let mode = c.i32()?;
    let stored = c.i32()?;
    c.i32()?; // IGES level
    let material = c.i32()?;
    c.i32()?; // (obsolete)
    let color = c.color()?;
    c.take(20)?; // obsolete line style
    let name = c.wstring()?;
    let mut layer = RhinoLayer { index: if stored >= 0 { stored } else { index }, name, color, visible: mode != 1, material, linetype: -1, section_style: -1, ..RhinoLayer::default() };
    if minor >= 1 {
        layer.visible = c.bool()? && mode != 1;
    }
    if minor >= 2 {
        layer.linetype = c.i32()?;
    }
    if minor >= 3 {
        // an unset print colour is the layer's colour (ON_Layer::PlotColor)
        layer.plot_color = Some(c.color()?).filter(|p| *p != [255; 4]).unwrap_or(color);
        layer.plot_weight = c.f64()?;
    }
    if minor >= 4 {
        c.bool()?; // locked
    }
    if minor >= 5 {
        layer.id = c.uuid()?;
        if minor >= 6 && c.v.opennurbs > 200_505_110 {
            layer.parent = c.uuid()?;
            c.bool()?; // expanded
        }
    }
    if minor >= 7 {
        // what follows is read as far as it goes: a short record keeps what it has
        let _ = read_layer_items(&mut c, minor, &mut layer);
    }
    Ok(layer)
}

/// The rest of `ON_Layer` (1.7 and later): its rendering attributes, display material (1.8),
/// two obsolete bytes (only 1.9), then items (1.10 and later; 0 ends them): the obsolete
/// selective clipping (28), section hatch index (29), scale (30) and rotation (31), the section
/// fill rule (32, 1.12), an obsolete linetype (33, 1.13), visible in new details (34, 1.14), its
/// own section style (35, 1.15), an obsolete list type (36); Rhino 9 adds the section style of
/// the table by index (39).
fn read_layer_items(c: &mut Cur<'_>, minor: u8, layer: &mut RhinoLayer) -> R<()> {
    c.chunk()?; // rendering attributes
    if minor < 8 {
        return Ok(());
    }
    c.uuid()?; // display material
    if minor == 9 {
        c.take(2)?;
    }
    if minor < 10 {
        return Ok(());
    }
    loop {
        match c.u8()? {
            28 => {
                c.bool()?;
                c.chunk()?;
            }
            29 => own_section(&mut layer.custom_section_style).hatch_index = c.i32()?,
            30 => own_section(&mut layer.custom_section_style).hatch_scale = c.f64()?,
            31 => own_section(&mut layer.custom_section_style).hatch_rotation = c.f64()?,
            32 => own_section(&mut layer.custom_section_style).fill_rule = c.u8()?.min(1),
            33 => {
                c.chunk()?;
            }
            34 => {
                c.bool()?;
            }
            35 => *own_section(&mut layer.custom_section_style) = styles::read_section_style(c, -1)?,
            36 => {
                c.bool()?;
            }
            // Rhino 9: the section style of the table, by index
            39 => layer.section_style = c.i32()?,
            _ => break,
        }
    }
    Ok(())
}

/// `ON_Material`: colours, shine, reflectivity, index of refraction and transparency, the
/// textures (of Rhino 3 its bitmap, bump and environment files), and the physically based
/// parameters of its user data.
fn read_material(object: Object<'_>, index: i32) -> R<Material> {
    let mut c = object.data;
    let mut m = Material {
        index,
        diffuse: [255, 255, 255, 0],
        specular: [255, 255, 255, 0],
        reflection: [255, 255, 255, 0],
        transparent: [255, 255, 255, 0],
        index_of_refraction: 1.0,
        ..Material::default()
    };
    m.pbr = object.userdata.iter().find(|(class, _)| *class == ON_PHYSICALLY_BASED_MATERIAL_USER_DATA).and_then(|(_, d)| read_pbr(d.clone()).ok());
    if c.v.archive >= 60 && c.v.opennurbs >= 2_348_833_910 {
        // Rhino 6 and later: component attributes, then the colours
        let (mut b, major, _) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
        if major != 1 {
            return err("a material of an unknown version");
        }
        let (mut attributes, _, _) = b.versioned(TCODE_ANONYMOUS_CHUNK)?;
        let bits = attributes.u32()?;
        if bits & 1 != 0 {
            m.id = attributes.uuid()?;
        }
        if bits & 2 != 0 {
            attributes.uuid()?;
        }
        if bits & 4 != 0 {
            m.index = attributes.i32()?;
        }
        if bits & 8 != 0 {
            m.name = attributes.wstring()?;
        }
        m.plug_in = b.uuid()?;
        read_material_colors(&mut b, &mut m)?;
        // Rhino reads these colours without their alpha
        for color in [&mut m.ambient, &mut m.diffuse, &mut m.emission, &mut m.specular, &mut m.reflection, &mut m.transparent] {
            color[3] = 0;
        }
        m.textures = read_textures(&mut b)?;
        // material channels, shareable, lighting, Fresnel and glossiness, then the render material;
        // a record that ends before them keeps what it has
        let mut rest = || -> R<Uuid> {
            let channels = b.count(20)?;
            b.take(20 * channels + 3 + 3 * 8)?;
            b.uuid()
        };
        m.render_material = rest().unwrap_or_default();
        if m.render_material.is_nil() {
            m.render_material = render::obsolete_material_instance(&object.userdata).unwrap_or_default();
        }
        return Ok(m);
    }
    let (major, minor) = c.version()?;
    match major {
        1 => {
            // Rhino 2 and 3
            m.ambient = c.color()?;
            m.diffuse = c.color()?;
            m.emission = c.color()?;
            m.specular = c.color()?;
            m.shine = c.f64()?;
            m.transparency = c.f64()?;
            c.take(4 + 4 + 2 + 2 + 16)?; // shadows, wires, wire colour, line style
            // the bitmap, bump and environment textures by file name
            for kind in [1, 2, 86] {
                let file = c.wstring()?;
                let mode = c.i32()?;
                c.i32()?; // bitmap index
                if kind == 2 {
                    c.f64()?; // bump scale
                }
                if !file.is_empty() {
                    m.textures.push(Texture { file, kind, mode: if mode == 2 { 2 } else { 1 }, ..Texture::default() });
                }
            }
            c.i32()?; // index
            c.uuid()?; // plug-in
            c.wstring()?; // Flamingo library
            m.name = c.wstring()?;
            if minor >= 1 {
                c.uuid()?;
                m.reflection = c.color()?;
                m.transparent = c.color()?;
                m.index_of_refraction = c.f64()?;
            }
        }
        2 => {
            // Rhino 4 and 5
            let (mut b, _, minor) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
            m.id = b.uuid()?;
            m.index = b.i32()?;
            m.name = b.wstring()?;
            m.plug_in = b.uuid()?;
            read_material_colors(&mut b, &mut m)?;
            // before December 2009 the default transparent colour was grey: Rhino takes the diffuse
            if c.v.opennurbs < 200_912_010 && m.transparent[..3] == [128, 128, 128] {
                m.transparent = m.diffuse;
            }
            m.textures = read_textures(&mut b)?;
            // Flamingo library (1.1), material channels (1.2), shareable and lighting (1.3),
            // Fresnel and glossiness (1.4), then the render material (1.5)
            if minor >= 5 {
                let mut rest = || -> R<Uuid> {
                    b.wstring()?;
                    let channels = b.count(20)?;
                    b.take(20 * channels + 3 + 3 * 8)?;
                    b.uuid()
                };
                m.render_material = rest().unwrap_or_default();
            }
        }
        _ => return err("a material of an unknown version"),
    }
    // files before Rhino 6 keep the render material in RDK user data
    if m.render_material.is_nil() {
        m.render_material = render::obsolete_material_instance(&object.userdata).unwrap_or_default();
    }
    Ok(m)
}

/// The colours (ambient, diffuse, emission, specular, reflection, transparent), index of
/// refraction, reflectivity, shine and transparency of a Rhino 4 and later `ON_Material`.
fn read_material_colors(c: &mut Cur<'_>, m: &mut Material) -> R<()> {
    m.ambient = c.color()?;
    m.diffuse = c.color()?;
    m.emission = c.color()?;
    m.specular = c.color()?;
    m.reflection = c.color()?;
    m.transparent = c.color()?;
    m.index_of_refraction = c.f64()?;
    m.reflectivity = c.f64()?;
    m.shine = c.f64()?;
    m.transparency = c.f64()?;
    Ok(())
}

const ON_TEXTURE: Uuid = Uuid::parse("D6FF106D-329B-4f29-97E2-FD282A618020");
const ON_PHYSICALLY_BASED_MATERIAL_USER_DATA: Uuid = Uuid::parse("5694E1AC-40E6-44F4-9CA9-3B6D0E8C4440");
/// At most this many textures are read of a material.
const MAX_TEXTURES: usize = 1000;

/// The texture array of a Rhino 4 and later material: a count and as many `ON_Texture` objects.
fn read_textures(c: &mut Cur<'_>) -> R<Vec<Texture>> {
    let (mut b, major, _) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    if major != 1 {
        return Ok(Vec::new());
    }
    // every texture is an object chunk of at least 12 bytes
    let count = b.count(12)?.min(MAX_TEXTURES);
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if let Some(object) = b.object()?.filter(|o| o.class == ON_TEXTURE) {
            out.push(read_texture(object.data)?);
        }
    }
    Ok(out)
}

/// `ON_Texture` (chunk version 1.x): file, type, mode, the transform of its coordinates and the
/// blend constant.
fn read_texture(mut c: Cur<'_>) -> R<Texture> {
    let (mut b, major, _) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    if major != 1 {
        return err("a texture of an unknown version");
    }
    b.uuid()?;
    let mapping_channel = b.i32()?;
    let file = b.wstring()?;
    let enabled = b.bool()?;
    let kind = b.u32()?;
    let mode = b.u32()?;
    b.take(5 * 4)?; // filters and wrapping
    let uvw = b.xform()?;
    b.take(4 + 4 + 16 + 16)?; // border and transparent colour, transparency texture, bump scale
    let amount = b.f64()?;
    Ok(Texture { file, kind, enabled, mode, amount, uvw, mapping_channel })
}

/// `ON_PhysicallyBasedMaterialUserData` (chunk version 1.1 or 1.2): the parameters.
fn read_pbr(mut c: Cur<'_>) -> R<Pbr> {
    let (mut b, major, minor) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    if major != 1 || !(1..=2).contains(&minor) {
        return err("physically based material data of an unknown version");
    }
    // `ON_4fColor`: red, green, blue and alpha floats
    let color = |b: &mut Cur<'_>| -> R<[f32; 4]> {
        let rgba = b.take(16)?;
        Ok(std::array::from_fn(|k| f32::from_le_bytes(rgba[4 * k..4 * k + 4].try_into().unwrap())))
    };
    Ok(Pbr {
        base_color: color(&mut b)?,
        brdf: b.i32()?,
        subsurface: b.f64()?,
        subsurface_scattering_color: color(&mut b)?,
        subsurface_scattering_radius: b.f64()?,
        metallic: b.f64()?,
        specular: b.f64()?,
        specular_tint: b.f64()?,
        roughness: b.f64()?,
        anisotropic: b.f64()?,
        anisotropic_rotation: b.f64()?,
        sheen: b.f64()?,
        sheen_tint: b.f64()?,
        clearcoat: b.f64()?,
        clearcoat_roughness: b.f64()?,
        opacity_ior: b.f64()?,
        opacity: b.f64()?,
        opacity_roughness: b.f64()?,
        emission: color(&mut b)?,
        alpha: if minor >= 2 { b.f64()? } else { 1.0 },
    })
}

/// `TCODE_BITMAP_TABLE` and its records.
const TCODE_BITMAP_TABLE: u32 = 0x1000_0016;
const TCODE_BITMAP_RECORD: u32 = 0x2000_8090;
const ON_EMBEDDED_BITMAP: Uuid = Uuid::parse("772E6FC1-B17B-4fc4-8F54-5FDA511D76D2");
const ON_WINDOWS_BITMAP_EX: Uuid = Uuid::parse("203AFC17-BCC9-44fb-A07B-7F5C31BD5ED9");
/// At most this many bytes of an embedded image are read.
const MAX_BITMAP: usize = 1 << 30;

/// An image of the bitmap table: of an `ON_EmbeddedBitmap` its file name, bytes and (chunk
/// version 1.1) name; of an `ON_WindowsBitmapEx` its file name. `None` for other bitmaps.
fn read_bitmap(object: Object<'_>) -> R<Option<Bitmap>> {
    let mut c = object.data;
    if object.class == ON_WINDOWS_BITMAP_EX {
        let (major, _) = c.version()?;
        if major != 1 {
            return Ok(None);
        }
        return Ok(Some(Bitmap { file: c.wstring()?, ..Bitmap::default() }));
    }
    if object.class != ON_EMBEDDED_BITMAP {
        return Ok(None);
    }
    let (major, minor) = c.version()?;
    if major != 1 {
        return err("an embedded bitmap of an unknown version");
    }
    let file = c.wstring()?;
    c.u32()?; // CRC
    let data = match c.i32()? {
        0 => {
            let size = c.u32()? as usize;
            c.take(size)?.to_vec()
        }
        1 => {
            // the size comes first: read the bytes only when they are not too many
            let size = c.peek_typecode().unwrap_or(0) as usize;
            if size > MAX_BITMAP {
                return err("an embedded bitmap larger than the reader reads");
            }
            c.compressed(size)?.unwrap_or_default()
        }
        _ => return err("an embedded bitmap of an unknown kind"),
    };
    let name = if minor >= 1 {
        c.uuid()?;
        c.wstring()?
    } else {
        String::new()
    };
    Ok(Some(Bitmap { name, file, data: Some(data) }))
}

/// `ON_InstanceDefinition`: id, name and the ids of its objects.
fn read_block_definition(mut c: Cur<'_>) -> R<BlockDefinition> {
    let v6 = match c.v.archive {
        0..=50 => false,
        60 => c.v.opennurbs > 2_348_834_153 && c.peek_typecode() == Some(TCODE_ANONYMOUS_CHUNK),
        _ => true,
    };
    if !v6 {
        let (major, _) = c.version()?;
        if major != 1 {
            return err("a block definition of an unknown version");
        }
        let id = c.uuid()?;
        let count = c.count(16)?;
        let objects = (0..count).map(|_| c.uuid()).collect::<R<Vec<_>>>()?;
        let name = c.wstring()?;
        return Ok(BlockDefinition { id, name, objects });
    }
    let (mut b, major, _) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    if major != 1 {
        return err("a block definition of an unknown version");
    }
    let (mut a, _, _) = b.versioned(TCODE_MODEL_ATTRIBUTES_CHUNK)?;
    let mut definition = BlockDefinition { id: Uuid::default(), name: String::new(), objects: Vec::new() };
    if a.u8()? == 1 {
        a.take(12)?; // serial numbers
    }
    if a.u8()? == 1 {
        definition.id = a.uuid()?;
    }
    if a.u8()? == 1 {
        a.i32()?; // component type
    }
    if a.u8()? == 1 {
        a.i32()?; // index
    }
    if a.u8()? == 1 {
        definition.name = a.wstring()?;
    }
    b.i32()?; // definition type (static, linked, ..)
    b.chunk()?; // unit system
    for _ in 0..3 {
        b.wstring()?; // description, url, url tag
    }
    b.take(48)?; // bounding box
    if b.bool()? {
        let count = b.count(16)?;
        definition.objects = (0..count).map(|_| b.uuid()).collect::<R<Vec<_>>>()?;
    }
    Ok(definition)
}

/// One record of the object table: the object type, the object, its attributes and the user
/// data of the attributes. `vertices` counts the mesh vertices read from the file so far.
fn read_object<'a>(mut c: Cur<'a>, vertices: &mut usize, drafting: &Drafting) -> R<RhinoObject<'a>> {
    let kind = c.chunk()?;
    if kind.typecode != TCODE_OBJECT_RECORD_TYPE {
        return err("an object record without its type");
    }
    let object_type = kind.value as u32;
    let object = c.object()?;
    let mut attributes = Attributes::default();
    let mut user = Vec::new();
    let (mut decals, mut modifiers) = (Vec::new(), threedm_model::MeshModifiers::default());
    while c.left() > 0 {
        let chunk = c.chunk()?;
        match chunk.typecode {
            TCODE_OBJECT_RECORD_ATTRIBUTES => attributes = read_attributes(chunk.body)?,
            TCODE_OBJECT_RECORD_ATTRIBUTES_USERDATA => {
                let mut body = chunk.body;
                let userdata = user_data(&mut body);
                user.extend(user_strings_of(&userdata));
                render::object_render(&userdata, &mut decals, &mut modifiers);
            }
            TCODE_OBJECT_RECORD_END => break,
            _ => {}
        }
    }
    let font = object.as_ref().and_then(|o| fonts::object_font(o.class, o.data.clone(), &drafting.dim_styles, &drafting.fonts));
    // set in its own font when the caller has it
    drafting.font.set(font.as_ref().and_then(|f| drafting.resolve_font.and_then(|resolve| resolve(f))).or(drafting.default_font));
    let geometry = match object {
        Some(object) => {
            // a hatch's background fill and boundary are in its attributes
            let hatch = (object.class == drafting::ON_HATCH && (attributes.hatch_background.is_some() || attributes.hatch_boundary)).then(|| object.data.clone());
            user.extend(user_strings_of(&object.userdata));
            render::object_render(&object.userdata, &mut decals, &mut modifiers);
            if object.class == ON_TEXT_DOT {
                // its text, shown with its point
                user.extend(text_dot_text(object.data.clone()).unwrap_or_default());
            }
            if object.class == dimensions::V2_TEXT_DOT {
                if let Ok((_, text)) = dimensions::read_v2_text_dot(&mut object.data.clone()) {
                    user.push(("Text".to_string(), text));
                }
            }
            match read_geometry(object, object_type, vertices, drafting) {
                Ok(geometry) => match hatch.map(|mut c| drafting::read_hatch(&mut c)) {
                    Some(Ok(h)) => drafting::hatch_drawing(geometry, &h, attributes.hatch_background, attributes.hatch_boundary),
                    _ => geometry,
                },
                Err(e) if e == TOO_MANY_VERTICES => Geometry::Skipped("meshes beyond the vertex limit"),
                Err(_) => Geometry::Skipped("unreadable objects"),
            }
        }
        None => Geometry::Skipped("empty objects"),
    };
    (attributes.decals, attributes.mesh_modifiers) = (decals, modifiers);
    Ok(RhinoObject { geometry, attributes, user, stroke: None, font })
}

/// `ON_3dmObjectAttributes`: the item list of Rhino 5 and later, or the fixed layout before.
fn read_attributes(mut c: Cur<'_>) -> R<Attributes> {
    let mut a = Attributes::default();
    if c.v.archive >= 5 && c.v.opennurbs >= 200_712_190 {
        let (major, _) = c.version()?;
        if major != 2 {
            return err("object attributes of an unknown version");
        }
        a.id = c.uuid()?;
        a.layer = c.i32()?;
        // items in increasing order, 0 ends the list (and one this reader does not know)
        loop {
            let item = c.u8()?;
            match item {
                1 => a.name = c.wstring()?,
                2 => a.url = c.wstring()?,
                3 => a.linetype = c.i32()?,
                4 => a.material = c.i32()?,
                5 => {
                    c.chunk()?; // rendering attributes
                }
                6 => a.color = c.color()?,
                7 => a.plot_color = c.color()?,
                8 => a.plot_weight = c.f64()?,
                9 => {
                    c.u8()?; // decoration
                }
                10 => {
                    c.i32()?; // wire density
                }
                11 => a.visible = c.bool()?,
                12 => a.mode = c.u8()? % 16,
                13 => a.color_source = c.u8()?,
                14 => a.plot_color_source = c.u8()?,
                15 => a.plot_weight_source = c.u8()?,
                17 => a.linetype_source = c.u8()?,
                16 => a.material_source = c.u8()?,
                18 => {
                    let n = c.count(4)?;
                    a.groups = (0..n).map(|_| c.i32()).collect::<R<_>>()?;
                }
                19 => a.space = c.u8()?,
                20 => a.viewport = c.uuid()?,
                21 => {
                    let n = c.count(32)?;
                    c.take(32 * n)?; // display material references (viewport, material)
                }
                22 => a.display_order = c.i32()?,
                23..=27 => {
                    c.u8()?; // obsolete line cap and join, clip participation source
                }
                28 => {
                    c.bool()?; // clipping proof
                    let (mut list, _, _) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
                    let n = list.count(16)?;
                    list.take(16 * n)?; // the clipping planes that clip it
                }
                29 => a.section_source = c.u8()?,
                // section attributes before the section style (39): into the object's own
                30 => own_section(&mut a.custom_section_style).hatch_index = c.i32()?,
                31 => own_section(&mut a.custom_section_style).hatch_scale = c.f64()?,
                32 => own_section(&mut a.custom_section_style).hatch_rotation = c.f64()?,
                33 => a.linetype_scale = c.f64()?,
                34 => a.hatch_background = Some(c.color()?),
                35 => a.hatch_boundary = c.bool()?,
                36 => {
                    c.xform()?; // object frame
                }
                37 => own_section(&mut a.custom_section_style).fill_rule = c.u8()?.min(1),
                38 => {
                    // ON_Linetype: read from a copy, then passed by
                    if let Ok(lt) = styles::read_linetype(c.clone(), -1) {
                        a.custom_linetype = Some(lt.segments.clone());
                        a.custom_linetype_style = Some(lt);
                    }
                    c.chunk()?;
                }
                39 => a.custom_section_style = Some(styles::read_section_style(&mut c, -1)?),
                40 => a.clipping_label = c.u8()?,
                41 => {
                    c.bool()?; // obsolete clipping list type
                }
                // Rhino 9: the section style of the table, by index
                43 => a.section_style = c.i32()?,
                _ => break,
            }
        }
    } else {
        let (major, minor) = c.version()?;
        if major != 1 {
            return err("object attributes of an unknown version");
        }
        a.id = c.uuid()?;
        a.layer = c.i32()?;
        a.material = c.i32()?;
        a.color = c.color()?;
        c.take(20)?; // obsolete line style
        c.i32()?; // wire density
        a.mode = c.u8()? % 16;
        a.color_source = c.u8()?;
        a.linetype_source = c.u8()?;
        a.material_source = c.u8()?;
        a.name = c.wstring()?;
        c.wstring()?; // url
        a.visible = a.mode != 1;
        if minor >= 1 {
            let n = c.count(4)?;
            c.take(4 * n)?;
            if minor >= 2 {
                a.visible = c.bool()?;
            }
            // display material references (1.3), decoration and plot colour and weight (1.4),
            // the linetype (1.5), then the space (1.6): 1 for a layout. A short record keeps
            // what it has.
            let mut rest = || -> R<()> {
                if minor >= 3 {
                    let n = c.count(32)?;
                    c.take(32 * n)?;
                }
                if minor >= 4 {
                    c.take(4 + 1 + 4 + 1 + 8)?;
                }
                if minor >= 5 {
                    a.linetype = c.i32()?;
                }
                if minor >= 6 {
                    if c.u8()? == 1 {
                        a.space = 2;
                    }
                    // display material references again (viewport, material): the page an
                    // object is on is kept as one with an obsolete material id
                    let n = c.count(32)?;
                    for _ in 0..n {
                        let (viewport, material) = (c.uuid()?, c.uuid()?);
                        if material == ON_OBSOLETE_PAGE_SPACE_OBJECT {
                            a.viewport = viewport;
                        }
                    }
                }
                Ok(())
            };
            let _ = rest();
        }
    }
    Ok(a)
}

/// The own section style of an object or layer, made when a section attribute comes before it.
fn own_section(style: &mut Option<SectionStyle>) -> &mut SectionStyle {
    style.get_or_insert_with(SectionStyle::default)
}

/// `ON_ObsoletePageSpaceObjectId`: the display material of the reference that puts an object
/// on a layout, in attributes before the item list.
const ON_OBSOLETE_PAGE_SPACE_OBJECT: Uuid = Uuid::parse("9BBB37E9-2131-4fb8-B9C6-5524859B98B8");

fn read_geometry<'a>(object: Object<'a>, object_type: u32, vertices: &mut usize, d: &Drafting) -> R<Geometry<'a>> {
    let mut c = object.data;
    let class = object.class;
    if class == ON_MESH {
        return Ok(Geometry::Meshes(vec![read_mesh(&mut c, vertices)?], "Mesh"));
    }
    if class == ON_BREP || BREP_ALIASES.contains(&class) {
        let (meshes, faces) = read_brep(&mut c, vertices)?;
        let kind = if faces == 1 { "Surface" } else { "Polysurface" };
        return Ok(if meshes.is_empty() { Geometry::Skipped("polysurfaces without a render mesh") } else { Geometry::Meshes(meshes, kind) });
    }
    if class == ON_EXTRUSION {
        let (mut meshes, shape, minor) = read_extrusion(&mut c, vertices)?;
        if meshes.is_empty() {
            // Rhino 5: the meshes are user data of the extrusion
            if let Some((_, data)) = object.userdata.iter().find(|(id, _)| *id == ON_V5_EXTRUSION_MESH_CACHE) {
                let mut data = data.clone();
                for _ in 0..2 {
                    match data.object() {
                        Ok(Some(o)) if o.class == ON_MESH => {
                            let mut d = o.data;
                            meshes.push(read_mesh(&mut d, vertices)?);
                            break;
                        }
                        Ok(_) => {}
                        Err(_) => break,
                    }
                }
            }
        }
        if meshes.is_empty() {
            // no mesh in the file: walls and caps from the profile and path
            match tessellate::extrusion(&shape, minor, vertices) {
                Err(e) if e == TOO_MANY_VERTICES => return Err(e),
                mesh => meshes.extend(mesh.ok().flatten()),
            }
        }
        return Ok(if meshes.is_empty() { Geometry::Skipped("extrusions without a render mesh") } else { Geometry::Meshes(meshes, "Extrusion") });
    }
    if tessellate::is_surface(class) {
        return Ok(match tessellate::surface_object(class, c, vertices) {
            Ok(Some(mesh)) => Geometry::Meshes(vec![mesh], "Surface"),
            Err(e) if e == TOO_MANY_VERTICES => return Err(e),
            _ => Geometry::Skipped("surfaces without a mesh"),
        });
    }
    if class == ON_INSTANCE_REF {
        let (major, _) = c.version()?;
        if major != 1 {
            return err("a block instance of an unknown version");
        }
        let definition = c.uuid()?;
        let xform = c.xform()?;
        return Ok(Geometry::Instance { definition, xform });
    }
    if class == ON_POINT_CLOUD {
        let (major, minor) = c.version()?;
        if major != 1 {
            return err("a point cloud of an unknown version");
        }
        let count = c.count(24)?;
        let points = c.take(24 * count)?;
        c.take(16 * 8 + 48 + 4)?; // plane, bounding box, flags
        let mut colors = None;
        if minor >= 1 {
            let normals = c.count(24)?;
            c.take(24 * normals)?;
            let n = c.count(4)?;
            let list = c.take(4 * n)?;
            colors = (n == count).then_some(list);
        }
        return Ok(Geometry::Points { points, colors, kind: "Point cloud" });
    }
    if class == ON_TEXT_DOT {
        // a label Rhino draws in screen pixels: its point, its text in the properties
        c.version()?;
        return Ok(Geometry::Points { points: c.take(24)?, colors: None, kind: "Text dot" });
    }
    if class == ON_POINT {
        c.version()?;
        return Ok(Geometry::Points { points: c.take(24)?, colors: None, kind: "Point" });
    }
    if class == ON_SUBD {
        // Rhino keeps no mesh of a SubD: its limit surface, from the control net
        return Ok(match subd::subd(c, vertices) {
            Ok(Some(mesh)) => Geometry::Meshes(vec![mesh], "SubD"),
            Err(e) if e == TOO_MANY_VERTICES => return Err(e),
            _ => Geometry::Skipped("SubD objects that could not be read"),
        });
    }
    if class == drafting::ON_TEXT {
        return Ok(match drafting::read_text(&mut c) {
            Ok(text) => {
                // in its dimension style's text height and model space scale
                let style = d.dim_styles.iter().find(|s| s.id == text.style()).or(d.dim_styles.first());
                match d.font.get() {
                    Some(font) => Geometry::Meshes(text.mesh(style, font).into_iter().collect(), "Text"),
                    None => Geometry::Skipped("texts (no font to set them in)"),
                }
            }
            Err(_) => Geometry::Skipped("texts that could not be read"),
        });
    }
    if class == drafting::ON_HATCH {
        return Ok(match drafting::read_hatch(&mut c) {
            Ok(h) => {
                // in its pattern's lines, or filled (a solid or gradient pattern, or none found)
                match d.patterns.iter().find(|p| p.index == h.pattern()).filter(|p| !p.solid) {
                    Some(p) => Geometry::Lines(h.lines(p), "Hatch"),
                    None => Geometry::Meshes(h.fill().into_iter().collect(), "Hatch"),
                }
            }
            Err(_) => Geometry::Skipped("hatches that could not be read"),
        });
    }
    if class == dimensions::V2_ARROW {
        return Ok(match dimensions::read_v2_arrow(&mut c) {
            Ok(line) => Geometry::Lines(vec![line], "Curve"),
            Err(_) => Geometry::Skipped("Rhino 2 arrows that could not be read"),
        });
    }
    if dimensions::V2_ANNOTATIONS.contains(&class) {
        let style = d.document_style.clone().unwrap_or_default();
        return Ok(match dimensions::read_v2(&mut c, &style, d.font.get().unwrap_or(&[]), d.meters) {
            Ok(drawing) if drawing.kind == "Text" => Geometry::Meshes(drawing.meshes, "Text"),
            Ok(drawing) => Geometry::Drawing { lines: drawing.lines, meshes: drawing.meshes, kind: drawing.kind },
            Err(_) => Geometry::Skipped("Rhino 2 annotations that could not be read"),
        });
    }
    if dimensions::V5_ANNOTATIONS.contains(&class) {
        return Ok(match dimensions::read_v5(&mut c, &d.dim_styles, d.font.get().unwrap_or(&[]), d.meters) {
            Ok(drawing) if drawing.kind == "Text" => Geometry::Meshes(drawing.meshes, "Text"),
            Ok(drawing) => Geometry::Drawing { lines: drawing.lines, meshes: drawing.meshes, kind: drawing.kind },
            Err(_) => Geometry::Skipped("Rhino 5 annotations that could not be read"),
        });
    }
    if class == dimensions::V2_TEXT_DOT {
        return Ok(match dimensions::read_v2_text_dot(&mut c) {
            Ok((points, _)) => Geometry::Points { points, colors: None, kind: "Text dot" },
            Err(_) => Geometry::Skipped("text dots that could not be read"),
        });
    }
    if dimensions::is_annotation(class) {
        return Ok(match dimensions::read(class, &mut c, &d.dim_styles, d.font.get().unwrap_or(&[]), d.meters) {
            Ok(drawing) => Geometry::Drawing { lines: drawing.lines, meshes: drawing.meshes, kind: drawing.kind },
            Err(_) => Geometry::Skipped("dimensions and leaders that could not be read"),
        });
    }
    if let Some(polyline) = tessellate::curve_object(class, &mut c) {
        return Ok(Geometry::Lines(vec![polyline], "Curve"));
    }
    if class == ON_CLIPPING_PLANE_SURFACE {
        return Ok(match read_clipping_plane(&mut c) {
            Ok(plane) => Geometry::ClippingPlane(plane),
            Err(_) => Geometry::Skipped("clipping planes that could not be read"),
        });
    }
    if class == ON_DETAIL_VIEW {
        return Ok(match read_detail(&mut c) {
            Ok(detail) => detail,
            Err(_) => Geometry::Skipped("details that could not be read"),
        });
    }
    Ok(Geometry::Skipped(match object_type {
        1 | 2 => "points",
        4 => "curves",
        8 => "surfaces without a mesh",
        0x200 => "annotations",
        0x2000 => "text dots",
        0x8000 => "details",
        0x10000 => "hatches",
        0x100 => "lights",
        0x2000_0000 => "clipping planes",
        0x40000 => "SubD objects (no mesh in the file)",
        _ => "other objects",
    }))
}

const ON_DETAIL_VIEW: Uuid = Uuid::parse("C8C66EFA-B3CB-4e00-9440-2AD66203379E");

/// `ON_DetailView::Read`: in a chunk, its `ON_3dmView` (the model view it shows) and its
/// boundary (an `ON_NurbsCurve` on the page), each in a chunk of its own, then (1.1) the page
/// to model ratio. Whether the detail is locked is its view's locked projection.
fn read_detail<'a>(c: &mut Cur<'_>) -> R<Geometry<'a>> {
    let (mut b, major, minor) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    if major != 1 {
        return err("a detail of an unknown version");
    }
    let (view, _, _) = b.versioned(TCODE_ANONYMOUS_CHUNK)?;
    let (view, locked) = read_view(view)?.ok_or("a detail without a viewport")?;
    let (mut curve, _, _) = b.versioned(TCODE_ANONYMOUS_CHUNK)?;
    let boundary = tessellate::curve_object(tessellate::ON_NURBS_CURVE[0], &mut curve).ok_or("a detail without a boundary")?;
    // a perspective detail has no scale (Rhino gives 0), whatever is stored
    let scale = if minor >= 1 { Some(b.f64()?).filter(|s| s.is_finite() && *s > 0.0 && !view.perspective) } else { None };
    Ok(Geometry::Detail { boundary, view, scale, locked })
}

const ON_CLIPPING_PLANE_SURFACE: Uuid = Uuid::parse("DBC5A584-CE3F-4170-98A8-497069CA5C36");

/// `ON_ClippingPlaneSurface::Read`: its plane surface (the rectangle Rhino draws), then
/// `ON_ClippingPlane`: a viewport id, the plane's id, the plane, enabled, the viewports it clips
/// (1.1 and later), its depth (1.2; whether the depth is used from 1.4) and the objects and
/// layers it clips (1.5: items after a type byte, 0 ends them).
fn read_clipping_plane(c: &mut Cur<'_>) -> R<ClippingPlane> {
    let (mut b, major, _) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    if major != 1 {
        return err("a clipping plane of an unknown version");
    }
    let mut surface = b.chunk()?.body;
    let corners = tessellate::plane_rectangle(&mut surface)?;
    let (mut p, major, minor) = b.versioned(TCODE_ANONYMOUS_CHUNK)?;
    if major != 1 {
        return err("a clipping plane of an unknown version");
    }
    let mut viewports = vec![p.uuid()?];
    p.uuid()?; // the plane's id
    let point = |p: &mut Cur<'_>| -> R<DVec3> { Ok(DVec3::new(p.f64()?, p.f64()?, p.f64()?)) };
    let origin = point(&mut p)?;
    p.take(48)?; // x and y axes
    let normal = point(&mut p)?.normalize_or_zero();
    p.take(32)?; // plane equation
    let enabled = p.bool()?;
    if minor >= 1 {
        let (mut list, _, _) = p.versioned(TCODE_ANONYMOUS_CHUNK)?;
        let n = list.count(16)?;
        viewports = (0..n).map(|_| list.uuid()).collect::<R<_>>()?;
    }
    let mut depth = None;
    if minor >= 2 {
        let d = p.f64()?;
        let used = if minor >= 4 { p.bool()? } else { d >= 0.0 && d < 1e38 };
        if used && d.is_finite() && d >= 0.0 {
            depth = Some(d);
        }
    }
    let mut selection = None;
    if minor >= 5 {
        let (mut chosen, mut lists_enabled) = (ClipSelection::default(), true);
        loop {
            match p.u8()? {
                10 => {
                    let n = p.count(16)?;
                    chosen.objects = (0..n).map(|_| p.uuid()).collect::<R<_>>()?;
                }
                11 => {
                    let n = p.count(4)?;
                    chosen.layers = (0..n).map(|_| p.i32()).collect::<R<_>>()?;
                }
                12 => chosen.except = p.bool()?,
                13 => lists_enabled = p.bool()?,
                _ => break,
            }
        }
        if lists_enabled && (!chosen.objects.is_empty() || !chosen.layers.is_empty()) {
            selection = Some(chosen);
        }
    }
    if normal == DVec3::ZERO || !origin.is_finite() {
        return err("a clipping plane without a plane");
    }
    Ok(ClippingPlane { corners, origin, normal, enabled, viewports, depth, selection })
}

const TCODE_GROUP_TABLE: u32 = 0x1000_0018;
const TCODE_GROUP_RECORD: u32 = 0x2000_8073;
const ON_GROUP: Uuid = Uuid::parse("721D9F97-3645-44c4-8BE6-B2CF697D25CE");

/// `ON_Group::Read` (version 1): its index and name (and, from 1.1, its id).
fn read_group(mut c: Cur<'_>) -> R<(i32, String)> {
    let (major, _) = c.version()?;
    if major != 1 {
        return err("a group of an unknown version");
    }
    Ok((c.i32()?, c.wstring()?))
}

/// At most this many dashes are made of one curve; beyond, it is drawn whole.
const MAX_DASHES: usize = 100_000;

/// Draws a curve in its linetype (by object, or by layer): its polylines cut into the
/// linetype's dashes, half a first dash at each end as Rhino does. Dots (dashes of length 0)
/// are a tenth of the pattern's smallest gap long.
fn dash(object: &mut RhinoObject<'_>, layers: &[RhinoLayer], linetypes: &[Linetype]) {
    let Geometry::Lines(polylines, "Curve") = &object.geometry else { return };
    let a = &object.attributes;
    let index = match a.linetype_source {
        1 => a.linetype,
        _ => layers.iter().find(|l| l.index == a.layer).map_or(-1, |l| l.linetype),
    };
    if index < 0 && a.custom_linetype.is_none() {
        return;
    }
    let scale = if a.linetype_scale.is_finite() && a.linetype_scale > 0.0 { a.linetype_scale } else { 1.0 };
    let segments: Vec<f64> = match &a.custom_linetype {
        Some(own) => own.iter().map(|s| s * scale).collect(),
        None => {
            let Some(lt) = linetypes.iter().find(|l| l.index == index) else { return };
            lt.segments.iter().map(|s| s * scale).collect()
        }
    };
    let segments = &segments;
    let period: f64 = segments.iter().map(|s| s.abs()).sum();
    if segments.len() < 2 || !segments.iter().any(|s| *s < 0.0) || !(period > 1e-12) {
        return;
    }
    let smallest_gap = segments.iter().filter(|s| **s < 0.0).map(|s| s.abs()).fold(f64::INFINITY, f64::min);
    let dot = (smallest_gap / 10.0).min(period / 20.0);
    let mut out: Vec<Vec<DVec3>> = Vec::new();
    for line in polylines {
        let length: f64 = line.windows(2).map(|w| w[0].distance(w[1])).sum();
        if !(length > 0.0) || length / period * segments.len() as f64 > MAX_DASHES as f64 {
            out.push(line.clone());
            continue;
        }
        // the point at a distance along the polyline
        let mut ends = vec![0.0];
        for w in line.windows(2) {
            ends.push(ends.last().unwrap() + w[0].distance(w[1]));
        }
        let at = |t: f64| -> DVec3 {
            let k = ends.partition_point(|&e| e < t).clamp(1, line.len() - 1);
            let (e0, e1) = (ends[k - 1], ends[k]);
            let f = if e1 > e0 { (t - e0) / (e1 - e0) } else { 0.0 };
            line[k - 1].lerp(line[k], f.clamp(0.0, 1.0))
        };
        // the pieces from `a` to `b`, along the polyline's own points
        let piece = |a: f64, b: f64| -> Vec<DVec3> {
            let mut p = vec![at(a)];
            p.extend(ends.iter().zip(line).filter(|(e, _)| **e > a && **e < b).map(|(_, q)| *q));
            p.push(at(b));
            p
        };
        let mut t = -segments[0].abs() / 2.0;
        'pattern: loop {
            for s in segments {
                let len = s.abs();
                if *s >= 0.0 {
                    let (a, b) = (t.max(0.0), (t + if len > 0.0 { len } else { dot }).min(length));
                    if b > a {
                        out.push(piece(a, b));
                    }
                }
                t += len;
                if t >= length {
                    break 'pattern;
                }
            }
        }
    }
    object.geometry = Geometry::Lines(out, "Curve");
}

/// `ON_TextDot`: a label at a point, drawn by Rhino in screen pixels.
const ON_TEXT_DOT: Uuid = Uuid::parse("74198302-CDF4-4f95-9609-6D684F22AB37");

/// The texts of an `ON_TextDot`, as properties: its text and its second text (on hovering).
fn text_dot_text(mut c: Cur<'_>) -> R<Vec<(String, String)>> {
    let (_, minor) = c.version()?;
    c.take(24)?; // the point
    c.i32()?; // height in points
    let text = c.wstring()?;
    c.wstring()?; // font
    c.i32()?; // display bits
    let mut out = vec![("Text".to_string(), text)];
    if minor >= 1 {
        let secondary = c.wstring()?;
        if !secondary.is_empty() {
            out.push(("Secondary text".to_string(), secondary));
        }
    }
    Ok(out)
}

/// At most this many mesh vertices are read from a file: compressed arrays could otherwise
/// inflate far beyond the size of the file. Meshes beyond are left out and counted.
pub const MAX_VERTICES: usize = 3 * if cfg!(target_arch = "wasm32") { 8_000_000 } else { 40_000_000 };
const TOO_MANY_VERTICES: &str = "Rhino 3dm: more mesh vertices than the reader reads";

/// `ON_Mesh` (chunk version 1.x uncompressed, 3.x compressed): vertices, normals, texture
/// coordinates, colours and faces. Curvatures, n-gons and double precision vertices are not read.
/// `vertices` counts the vertices read from the file so far; a mesh beyond [`MAX_VERTICES`] is
/// not read. A mesh without faces draws nothing and is not read either.
#[doc(hidden)]
pub fn read_mesh(c: &mut Cur<'_>, vertices: &mut usize) -> R<Mesh> {
    let (major, _) = c.version()?;
    if major != 1 && major != 3 {
        return err("a mesh of an unknown version");
    }
    let vcount = c.i32()?.max(0) as usize;
    let fcount = c.i32()?.max(0) as usize;
    if fcount == 0 {
        return Ok(Mesh::default());
    }
    if vcount > MAX_VERTICES - *vertices {
        return Err(TOO_MANY_VERTICES.into());
    }
    *vertices += vcount;
    // texture and surface domains, surface scale, vertex, normal and texture boxes, closed
    c.take(64 + 16 + 24 + 24 + 16 + 4)?;
    if c.bool()? {
        c.chunk()?; // mesh parameters
    }
    for _ in 0..4 {
        if c.bool()? {
            c.chunk()?; // curvature statistics
        }
    }
    let size = c.i32()?;
    if !matches!(size, 1 | 2 | 4) {
        return err("mesh faces of an unknown index size");
    }
    let bytes = c.take(fcount.checked_mul(4 * size as usize).ok_or("Rhino 3dm: too many mesh faces")?)?;
    let faces: Vec<[u32; 4]> = match size {
        1 => bytes.chunks_exact(4).map(|f| [f[0] as u32, f[1] as u32, f[2] as u32, f[3] as u32]).collect(),
        2 => bytes.chunks_exact(8).map(|f| std::array::from_fn(|k| u16::from_le_bytes([f[2 * k], f[2 * k + 1]]) as u32)).collect(),
        _ => bytes.chunks_exact(16).map(|f| std::array::from_fn(|k| u32::from_le_bytes(f[4 * k..4 * k + 4].try_into().unwrap()))).collect(),
    };
    let floats3 = |b: &[u8]| -> Vec<[f32; 3]> { b.chunks_exact(12).map(|p| std::array::from_fn(|k| f32::from_le_bytes(p[4 * k..4 * k + 4].try_into().unwrap()))).collect() };
    let floats2 = |b: &[u8]| -> Vec<[f32; 2]> { b.chunks_exact(8).map(|p| std::array::from_fn(|k| f32::from_le_bytes(p[4 * k..4 * k + 4].try_into().unwrap()))).collect() };
    let colors4 = |b: &[u8]| -> Vec<[u8; 4]> { b.chunks_exact(4).map(|p| [p[0], p[1], p[2], p[3]]).collect() };
    let triangles = faces.iter().map(|f| if f[2] == f[3] { 1 } else { 2 }).sum();
    let mut mesh = Mesh { faces, triangles, ..Mesh::default() };
    if major == 1 {
        let n = c.count(12)?;
        mesh.positions = floats3(c.take(12 * n)?);
        let n = c.count(12)?;
        mesh.normals = Some(floats3(c.take(12 * n)?));
        let n = c.count(8)?;
        mesh.texture_coordinates = Some(floats2(c.take(8 * n)?));
        let n = c.count(16)?;
        c.take(16 * n)?;
        let n = c.count(4)?;
        mesh.colors = Some(colors4(c.take(4 * n)?));
    } else if vcount > 0 {
        mesh.positions = floats3(&c.compressed(12 * vcount)?.unwrap_or_default());
        mesh.normals = c.compressed(12 * vcount)?.map(|b| floats3(&b));
        mesh.texture_coordinates = c.compressed(8 * vcount)?.map(|b| floats2(&b));
        // curvatures, skipped without inflating them
        c.compressed(0)?;
        mesh.colors = c.compressed(4 * vcount)?.map(|b| colors4(&b));
    }
    let n = mesh.positions.len();
    if n != vcount {
        return err("mesh vertices of the wrong size");
    }
    mesh.normals = mesh.normals.filter(|v| v.len() == n);
    mesh.colors = mesh.colors.filter(|v| v.len() == n);
    mesh.texture_coordinates = mesh.texture_coordinates.filter(|v| v.len() == n && n > 0);
    Ok(mesh)
}

/// `ON_Brep` (chunk version 3.x): the curves, surfaces and topology as chunks; the face count,
/// then the render meshes (or else the analysis meshes) of the faces. A Brep saved without
/// meshes is meshed from its surfaces and trims ([`tessellate::brep`]). Returns the meshes and
/// the face count.
fn read_brep(c: &mut Cur<'_>, vertices: &mut usize) -> R<(Vec<Mesh>, usize)> {
    let (major, minor) = c.version()?;
    if major == 2 {
        // Rhino 2: its own layout; a polysurface that cannot be read is left out
        let mut mesh = |c: &mut Cur<'_>, vertices: &mut usize| -> R<Option<Mesh>> {
            Ok(match c.object()? {
                Some(o) if o.class == ON_MESH => {
                    let mut d = o.data;
                    Some(read_mesh(&mut d, vertices)?)
                }
                _ => None,
            })
        };
        let _ = minor;
        return match tessellate::brep_v2(c, vertices, &mut mesh) {
            Err(e) if e == TOO_MANY_VERTICES => Err(e),
            r => Ok(r.unwrap_or_default()),
        };
    }
    if major != 3 {
        // Rhino 1 trimmed surfaces
        return Ok((Vec::new(), 0));
    }
    // 2d curves, 3d curves, surfaces, vertices, edges, trims, loops, faces
    let mut parts = Vec::with_capacity(8);
    for _ in 0..8 {
        parts.push(c.chunk()?.body);
    }
    let parts: [Cur<'_>; 8] = parts.try_into().map_err(|_| "Rhino 3dm: a damaged polysurface")?;
    let faces = parts[7].clone();
    let face_count = {
        let mut f = faces.clone();
        f.version()?;
        f.i32()?.max(0) as usize
    };
    let face_colors = brep_face_colors(faces).unwrap_or_default();
    let low = DVec3::new(c.f64()?, c.f64()?, c.f64()?);
    let high = DVec3::new(c.f64()?, c.f64()?, c.f64()?);
    let size = (high - low).length();
    // a Brep that cannot be meshed is left out, as one without meshes was before
    let computed = |vertices: &mut usize| match tessellate::brep(&parts, size, &face_colors, vertices) {
        Err(e) if e == TOO_MANY_VERTICES => Err(e),
        meshes => Ok((meshes.unwrap_or_default(), face_count)),
    };
    if minor < 1 {
        return computed(vertices);
    }
    let mut out = Vec::new();
    for _ in 0..2 {
        let mut list = c.chunk()?.body;
        for face in 0..face_count {
            if list.left() == 0 {
                break;
            }
            if list.bool()? {
                if let Some(object) = list.object()?.filter(|o| o.class == ON_MESH) {
                    let mut d = object.data;
                    let mut mesh = read_mesh(&mut d, vertices)?;
                    mesh.color = face_colors.get(face).copied().flatten();
                    out.push(mesh);
                }
            }
        }
        // the analysis meshes only when there are no render meshes
        if !out.is_empty() || c.left() == 0 {
            break;
        }
    }
    if out.is_empty() {
        return computed(vertices);
    }
    Ok((out, face_count))
}

/// The per-face colours of a Brep's face array (chunk version 1.2, Rhino 7 and later); `None`
/// for a face without one.
fn brep_face_colors(mut c: Cur<'_>) -> R<Vec<Option<[u8; 4]>>> {
    let (_, minor) = c.version()?;
    let count = c.count(20)?;
    for _ in 0..count {
        c.i32()?; // index
        let loops = c.count(4)?;
        c.take(4 * loops)?;
        c.take(12)?; // surface, reversed, material channel
    }
    if minor < 2 {
        return Ok(Vec::new());
    }
    c.take(16 * count)?; // face ids
    if !c.bool()? {
        return Ok(Vec::new());
    }
    (0..count).map(|_| c.color().map(|color| (color != [255; 4]).then_some(color))).collect()
}

/// `ON_Extrusion`: the profile and path (the shape, meshed when the file has no mesh) and, from
/// chunk version 1.3 (Rhino 6), the mesh cache, of which the render mesh (or else the first) is
/// taken.
fn read_extrusion(c: &mut Cur<'_>, vertices: &mut usize) -> R<(Vec<Mesh>, tessellate::Extrusion, i32)> {
    let (mut b, major, minor) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    if major != 1 {
        return err("an extrusion of an unknown version");
    }
    let profile = tessellate::read_curve_object(&mut b, 0).unwrap_or(None);
    let (shape, minor) = tessellate::read_extrusion_shape(&mut b, profile, minor)?;
    if minor < 3 {
        return Ok((Vec::new(), shape, minor));
    }
    let (mut cache, _, _) = b.versioned(TCODE_ANONYMOUS_CHUNK)?;
    let mut meshes: Vec<(Uuid, Mesh)> = Vec::new();
    while cache.u8()? == 1 {
        let (mut item, _, _) = cache.versioned(TCODE_ANONYMOUS_CHUNK)?;
        let id = item.uuid()?;
        if let Some(object) = item.object()?.filter(|o| o.class == ON_MESH) {
            let mut d = object.data;
            meshes.push((id, read_mesh(&mut d, vertices)?));
        }
    }
    let k = meshes.iter().position(|(id, _)| *id == RENDER_MESH_ID).unwrap_or(0);
    let meshes = if meshes.is_empty() { Vec::new() } else { vec![meshes.swap_remove(k).1] };
    Ok((meshes, shape, minor))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;
    use std::io::Write;
    use threedm_core::write::*;

    /// An `ON_Mesh` 3.x of one triangle whose vertex array is `positions` (size, CRC, method and
    /// bytes as `compressed` writes them), claiming `vcount` vertices.
    fn mesh_with(vcount: i32, positions: &[u8]) -> Vec<u8> {
        cat(&[&[0x30], &i32b(vcount), &i32b(1), &[0; 153], &i32b(4), &[0, 1, 2, 2].map(i32b).concat(), positions, &[0; 16]])
    }

    fn read_test_mesh(data: &[u8]) -> R<Mesh> {
        let mut c = Cur { data, pos: 0, v: Versions { archive: 70, opennurbs: 2_400_000_000 } };
        read_mesh(&mut c, &mut 0)
    }

    #[test]
    fn compressed_arrays_are_checked_before_inflating() {
        let w = W { big: true };
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        z.write_all(&vec![0; 1 << 20]).unwrap();
        let zeros = z.finish().unwrap();
        // a few kilobytes that claim almost 4 GiB for 3 vertices: not inflated
        let bomb = cat(&[&0xFFFF_0000u32.to_le_bytes(), &[0; 4], &[1], &w.chunk(TCODE_ANONYMOUS_CHUNK, &zeros)]);
        assert_eq!(read_test_mesh(&mesh_with(3, &bomb)).unwrap_err(), "Rhino 3dm: mesh vertices of the wrong size");
        // a size that matches the vertex count but more than the stream can inflate to
        let packed = cat(&[&(12u32 * 1_000_000).to_le_bytes(), &[0; 4], &[1], &w.chunk(TCODE_ANONYMOUS_CHUNK, &zeros[..64])]);
        assert_eq!(read_test_mesh(&mesh_with(1_000_000, &packed)).unwrap_err(), "Rhino 3dm: compressed data larger than its stream can hold");
        // more vertices than the viewer reads
        assert_eq!(read_test_mesh(&mesh_with(i32::MAX, &bomb)).unwrap_err(), TOO_MANY_VERTICES);
        // the well-formed mesh reads
        let points = w.compressed(&[[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]].iter().flatten().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>(), true);
        let mesh = read_test_mesh(&mesh_with(3, &points)).unwrap();
        assert_eq!((mesh.positions.len(), mesh.triangles), (3, 1));
        // no faces and a face index size that would overflow a 32-bit multiplication
        let empty = cat(&[&[0x30], &i32b(3), &i32b(0), &[0; 153], &i32b(i32::MAX)]);
        assert!(read_test_mesh(&empty).unwrap().faces.is_empty());
    }

    #[test]
    fn user_text_counts_need_room_for_their_entries() {
        // 1000 entries in 8000 bytes: each entry needs at least 16
        let w = W { big: false };
        let list = w.versioned(TCODE_ANONYMOUS_CHUNK, 1, 0, &cat(&[&i32b(1000), &vec![0; 8000]]));
        let c = Cur { data: &list, pos: 0, v: Versions { archive: 4, opennurbs: 0 } };
        assert_eq!(user_strings(c).unwrap_err(), "Rhino 3dm: a count larger than its record");
    }

    /// A polysurface as Rhino 2 wrote it (`ON_Brep::ReadOld200`): its curves and surface inline,
    /// its trims numbered in order. A unit square meshes into a face of area 1.
    #[test]
    fn rhino2_polysurfaces_are_read() {
        let w = W { big: false };
        let line_class = Uuid::parse("4ED7D4DB-E947-11d3-BFE5-0010830122F0");
        // ON_PolyCurve inline, of one line
        let poly = |a: [f64; 3], b: [f64; 3]| {
            let line = cat(&[&[0x10], &f64s(&a), &f64s(&b), &f64s(&[0.0, 1.0]), &i32b(3)]);
            cat(&[&[0x10], &i32b(1), &i32b(0), &i32b(0), &[0; 48], &i32b(2), &f64s(&[0.0, 1.0]), &w.object(line_class, &line, &[])])
        };
        let square = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]];
        let mut data = cat(&[&i32b(1), &i32b(4), &i32b(1), &i32b(4), &i32b(1), &[0; 48]]);
        for k in 0..4 {
            data.extend(poly(square[k], square[(k + 1) % 4])); // trims in the parameter plane
        }
        for k in 0..4 {
            data.extend(poly(square[k], square[(k + 1) % 4])); // edges
        }
        // ON_NurbsSurface inline: a bilinear patch over the unit square, u along x
        let cvs: Vec<f64> = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]].iter().flat_map(|[x, y]| [*x, *y, 0.0]).collect();
        data.extend(cat(&[&[0x10], &i32b(3), &i32b(0), &i32b(2), &i32b(2), &i32b(2), &i32b(2), &i32b(0), &i32b(0), &[0; 48], &i32b(2), &f64s(&[0.0, 1.0]), &i32b(2), &f64s(&[0.0, 1.0]), &i32b(4), &f64s(&cvs)]));
        // the face, its outer loop and its trims
        data.extend(cat(&[&i32b(0), &i32b(0), &i32b(0), &i32b(0), &[0; 48], &i32b(1), &i32b(0), &i32b(0), &[0; 32], &i32b(4)]));
        for k in 0..4 {
            data.extend(cat(&[&i32b(k), &i32b(-1), &[1], &i32b(k), &i32b(0), &i32b(0), &i32b(0), &f64s(&[0.0, 0.0])]));
        }
        data.push(0); // no render mesh
        let mut c = Cur { data: &data, pos: 0, v: Versions { archive: 2, opennurbs: 0 } };
        let mut vertices = 0;
        let (meshes, faces) = super::tessellate::brep_v2(&mut c, &mut vertices, &mut |_, _| Ok(None)).unwrap();
        assert_eq!((meshes.len(), faces), (1, 1));
        let m = &meshes[0];
        let p = |i: u32| DVec3::from(m.positions[i as usize].map(f64::from));
        let area: f64 = m.faces.iter().map(|f| 0.5 * (p(f[1]) - p(f[0])).cross(p(f[2]) - p(f[0])).length()).sum();
        assert!((area - 1.0).abs() < 1e-9, "{area}");
        assert_eq!(c.left(), 0, "the whole record is read");
    }

    /// A text dot (`ON_TextDot` 1.1): its point, drawn as a point, and its two texts as
    /// properties.
    #[test]
    fn text_dots_are_points_with_their_text() {
        let w = W { big: true };
        let data = cat(&[&[0x11], &f64s(&[1.0, 2.0, 3.0]), &i32b(12), &wstr("A1"), &wstr("Arial"), &i32b(0), &wstr("Grid A, 1")]);
        let bytes = w.object(ON_TEXT_DOT, &data, &[]);
        let mut c = Cur { data: &bytes, pos: 0, v: Versions { archive: 70, opennurbs: 2_400_000_000 } };
        let object = c.object().unwrap().unwrap();
        assert_eq!(text_dot_text(object.data.clone()).unwrap(), [("Text".to_string(), "A1".to_string()), ("Secondary text".to_string(), "Grid A, 1".to_string())]);
        let drafting = Drafting { patterns: Vec::new(), dim_styles: Vec::new(), font: std::cell::Cell::new(None), default_font: None, resolve_font: None, meters: 0.001, document_style: None, fonts: Vec::new() };
        let Geometry::Points { points, kind, .. } = read_geometry(object, 0x2000, &mut 0, &drafting).unwrap() else { panic!("not a point") };
        assert_eq!((points, kind), (&f64s(&[1.0, 2.0, 3.0])[..], "Text dot"));
    }
}
