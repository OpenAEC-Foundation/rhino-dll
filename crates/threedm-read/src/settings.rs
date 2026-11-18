//! The document settings beside the units and views (`ON_3dmSettings::Read`): the earth
//! anchor point and model base point, named and view construction planes, render and analysis
//! mesh settings, grid defaults, render settings, the current table entries, the model URL and
//! the plug-in list; the history records (`TCODE_HISTORYRECORD_TABLE`); and the tables plug-ins
//! save (`TCODE_USER_TABLE`), of them the document user text, the named layer states and the
//! snapshots read.

use super::{read_layer, read_viewport, Chunk, Cur, Uuid, R, ON_LAYER, TCODE_ANONYMOUS_CHUNK, TCODE_ENDOFTABLE, TCODE_LAYER_RECORD};
use super::{TCODE_SETTINGS_ATTRIBUTES, TCODE_SETTINGS_VIEW_LIST, TCODE_VIEW_RECORD, TCODE_VIEW_VIEWPORT};
use glam::DVec3;
use threedm_model::*;

const TCODE_SETTINGS_PLUGINLIST: u32 = 0x2000_8135;
const TCODE_SETTINGS_RENDERMESH: u32 = 0x2000_8032;
const TCODE_SETTINGS_ANALYSISMESH: u32 = 0x2000_8033;
const TCODE_SETTINGS_NAMED_CPLANE_LIST: u32 = 0x2000_8035;
const TCODE_SETTINGS_CURRENT_LAYER_INDEX: u32 = 0xA000_0038;
const TCODE_SETTINGS_CURRENT_MATERIAL_INDEX: u32 = 0x2000_8039;
const TCODE_SETTINGS_CURRENT_COLOR: u32 = 0x2000_803A;
const TCODE_SETTINGS_CURRENT_WIRE_DENSITY: u32 = 0xA000_003C;
const TCODE_SETTINGS_RENDER: u32 = 0x2000_803D;
const TCODE_SETTINGS_GRID_DEFAULTS: u32 = 0x2000_803F;
const TCODE_SETTINGS_MODEL_URL: u32 = 0x2000_8131;
const TCODE_SETTINGS_CURRENT_FONT_INDEX: u32 = 0xA000_0132;
const TCODE_SETTINGS_CURRENT_DIMSTYLE_INDEX: u32 = 0xA000_0133;
const TCODE_VIEW_CPLANE: u32 = 0x2000_813B;

pub(super) const TCODE_HISTORYRECORD_TABLE: u32 = 0x1000_0026;
pub(super) const TCODE_HISTORYRECORD_RECORD: u32 = 0x2000_807B;
pub(super) const ON_HISTORY_RECORD: Uuid = Uuid::parse("ECD0FD2F-2088-49dc-9641-9CF7A28FFA6B");

pub(super) const TCODE_USER_TABLE: u32 = 0x1000_0017;
const TCODE_USER_TABLE_UUID: u32 = 0x2000_8080;
const TCODE_USER_TABLE_RECORD_HEADER: u32 = 0x2000_8082;
const TCODE_USER_RECORD: u32 = 0x2000_0081;

/// The user table of the document user text: `ON_DocumentUserStringList`'s class id.
const DOCUMENT_USER_STRINGS: Uuid = Uuid::parse("06F3218E-F5EC-4f6c-B74C-14583F0ED7BC");
/// The plug-ins whose tables hold the named layer states and the snapshots.
const LAYER_STATES: Uuid = Uuid::parse("213A2661-A9EC-4AC4-867A-BC8CABE5AE29");
const SNAPSHOTS: Uuid = Uuid::parse("73B88F43-C32C-4306-93B5-1D0082FFFEE8");

/// At most this many plug-ins, construction planes, values of a history record, layer states
/// and layers in one are read.
const MAX_ITEMS: usize = 100_000;

fn point(c: &mut Cur<'_>) -> R<DVec3> {
    Ok(DVec3::new(c.f64()?, c.f64()?, c.f64()?))
}

fn items(c: &mut Cur<'_>, size: usize) -> R<usize> {
    let n = c.count(size)?;
    if n > MAX_ITEMS {
        return super::err("too many items");
    }
    Ok(n)
}

/// A chunk of the settings table that [`super::read_settings`] does not read itself.
pub(super) fn read_chunk(chunk: &Chunk<'_>, s: &mut Settings) {
    let c = chunk.body.clone();
    let index = || i32::try_from(chunk.value).ok().filter(|&i| i >= -1);
    let _ = (|| -> R<()> {
        match chunk.typecode {
            TCODE_SETTINGS_PLUGINLIST => s.plugins = read_plugins(c)?,
            TCODE_SETTINGS_RENDERMESH => s.render_mesh = Some(read_mesh_settings(&mut c.clone())?),
            TCODE_SETTINGS_ANALYSISMESH => s.analysis_mesh = Some(read_mesh_settings(&mut c.clone())?),
            TCODE_SETTINGS_NAMED_CPLANE_LIST => s.named_cplanes = read_named_cplanes(c)?,
            TCODE_SETTINGS_VIEW_LIST => s.view_cplanes = read_view_cplanes(c)?,
            TCODE_SETTINGS_CURRENT_LAYER_INDEX => s.current.layer_index = index(),
            TCODE_SETTINGS_CURRENT_FONT_INDEX => s.current.text_style_index = index(),
            TCODE_SETTINGS_CURRENT_DIMSTYLE_INDEX => s.current.dimension_style_index = index(),
            TCODE_SETTINGS_CURRENT_WIRE_DENSITY => s.current.wire_density = chunk.value.clamp(-2, i32::MAX as i64) as i32,
            TCODE_SETTINGS_CURRENT_MATERIAL_INDEX => {
                let mut c = c;
                s.current.material_index = Some(c.i32()?);
                s.current.material_source = c.i32()?;
            }
            TCODE_SETTINGS_CURRENT_COLOR => {
                let mut c = c;
                s.current.color = c.color()?;
                s.current.color_source = c.i32()?;
            }
            TCODE_SETTINGS_RENDER => s.render = Some(read_render_settings(c)?),
            TCODE_SETTINGS_GRID_DEFAULTS => s.grid = Some(read_grid_defaults(c)?),
            TCODE_SETTINGS_MODEL_URL => s.model_url = c.clone().wstring()?,
            TCODE_SETTINGS_ATTRIBUTES => read_attributes(c, s)?,
            _ => {}
        }
        Ok(())
    })();
}

/// `TCODE_SETTINGS_ATTRIBUTES`: version 1.0 the linetype display scale, the current plot colour,
/// linetype and their sources; 1.1 the page units (read by [`super::read_page_units`]); 1.2
/// the active view; 1.3 the model base point and the earth anchor point; 1.4 whether texture
/// bitmaps are saved in the file; 1.5 the IO settings; 1.6 the custom render mesh settings; 1.7
/// the current layer, material, linetype, text style, dimension style and hatch pattern by id.
fn read_attributes(mut c: Cur<'_>, s: &mut Settings) -> R<()> {
    let (major, minor) = c.version()?;
    if major != 1 {
        return Ok(());
    }
    s.linetype_display_scale = c.f64()?;
    s.current.plot_color = c.color()?;
    s.current.plot_color_source = c.i32()?;
    s.current.linetype_index = Some(c.i32()?);
    s.current.linetype_source = c.i32()?;
    if minor < 1 {
        return Ok(());
    }
    c.chunk()?; // page units and tolerances
    if minor < 2 {
        return Ok(());
    }
    s.active_view = c.uuid()?;
    if minor < 3 {
        return Ok(());
    }
    s.model_basepoint = point(&mut c)?;
    s.earth_anchor = Some(read_earth_anchor(c.chunk()?.body)?);
    if minor < 4 {
        return Ok(());
    }
    s.save_texture_bitmaps = c.bool()?;
    if minor < 5 {
        return Ok(());
    }
    // ON_3dmIOSettings: bitmaps in the file (again), how linked blocks update
    let (mut io, io_major, _) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    if io_major == 1 {
        s.save_texture_bitmaps = io.bool()?;
        s.idef_link_update = io.i32()?;
        if s.idef_link_update == 0 && c.v.archive >= 5 {
            s.idef_link_update = 1;
        }
    }
    if minor < 6 {
        return Ok(());
    }
    s.custom_render_mesh = Some(read_mesh_settings(&mut c)?);
    if minor < 7 {
        return Ok(());
    }
    let cur = &mut s.current;
    (cur.layer_index, cur.material_index, cur.linetype_index, cur.text_style_index, cur.dimension_style_index) = (None, None, None, None, None);
    cur.layer = c.uuid()?;
    cur.material = c.uuid()?;
    cur.linetype = c.uuid()?;
    cur.text_style = c.uuid()?;
    cur.dimension_style = c.uuid()?;
    cur.hatch_pattern = c.uuid()?;
    Ok(())
}

/// `ON_EarthAnchorPoint::Read`, the chunk's body: latitude, longitude, elevation (metres), the
/// model point, north and east; from 1.1 the elevation's zero, an id, name, description, URL
/// and URL tag; from 1.2 the earth coordinate system.
fn read_earth_anchor(mut c: Cur<'_>) -> R<EarthAnchor> {
    let major = c.i32()?;
    let minor = c.i32()?;
    if major != 1 {
        return super::err("an earth anchor point of an unknown version");
    }
    let mut e = EarthAnchor { latitude: c.f64()?, longitude: c.f64()?, elevation: c.f64()?, ..EarthAnchor::default() };
    e.model_point = point(&mut c)?;
    e.model_north = point(&mut c)?;
    e.model_east = point(&mut c)?;
    if minor >= 1 {
        // 0 ground level, 1 mean sea level, 2 the centre of the earth
        e.coordinate_system = match c.i32()? {
            0 => 1,
            1 => 2,
            2 => 3,
            _ => 0,
        };
        e.id = c.uuid()?;
        e.name = c.wstring()?;
        e.description = c.wstring()?;
        e.url = c.wstring()?;
        e.url_tag = c.wstring()?;
        if minor >= 2 {
            e.coordinate_system = match c.u32()? {
                v @ (0..=3 | 5 | 6) => v as u8,
                _ => 0,
            };
        }
    }
    Ok(e)
}

/// `ON_3dmConstructionPlane::Read`: a version, the plane (origin, axes, equation), grid and
/// snap spacing, grid line count and thick line frequency, the name; from 1.1 whether it is
/// depth buffered.
fn read_cplane(c: &mut Cur<'_>) -> R<ConstructionPlane> {
    let (major, minor) = c.version()?;
    if major != 1 {
        return super::err("a construction plane of an unknown version");
    }
    let mut p = ConstructionPlane { origin: point(c)?, x: point(c)?, y: point(c)?, z: point(c)?, ..ConstructionPlane::default() };
    c.take(32)?; // plane equation
    p.grid_spacing = c.f64()?;
    p.snap_spacing = c.f64()?;
    p.grid_line_count = c.i32()?;
    p.thick_frequency = c.i32()?;
    p.name = c.wstring()?;
    // openNURBS' default
    p.depth_buffered = true;
    if minor >= 1 {
        p.depth_buffered = c.bool()?;
    }
    Ok(p)
}

/// `TCODE_SETTINGS_NAMED_CPLANE_LIST`: a count and as many `TCODE_VIEW_CPLANE` chunks.
fn read_named_cplanes(mut c: Cur<'_>) -> R<Vec<ConstructionPlane>> {
    let n = items(&mut c, 12)?;
    let mut out = Vec::with_capacity(n.min(1024));
    for _ in 0..n {
        let chunk = c.chunk()?;
        if chunk.typecode != TCODE_VIEW_CPLANE {
            return super::err("a named construction plane of an unknown kind");
        }
        out.push(read_cplane(&mut chunk.body.clone())?);
    }
    Ok(out)
}

/// The construction plane of each view of `TCODE_SETTINGS_VIEW_LIST`, by its viewport's id.
fn read_view_cplanes(mut c: Cur<'_>) -> R<Vec<(Uuid, ConstructionPlane)>> {
    let n = items(&mut c, 12)?;
    let mut out = Vec::new();
    for _ in 0..n {
        let record = c.chunk()?;
        if record.typecode != TCODE_VIEW_RECORD {
            return super::err("a view record of an unknown kind");
        }
        let (mut body, mut cplane, mut id) = (record.body, None, None);
        while let Ok(chunk) = body.chunk() {
            match chunk.typecode {
                TCODE_ENDOFTABLE => break,
                TCODE_VIEW_CPLANE => cplane = read_cplane(&mut chunk.body.clone()).ok(),
                TCODE_VIEW_VIEWPORT => id = read_viewport(chunk.body).ok().map(|v| v.id),
                _ => {}
            }
        }
        if let (Some(id), Some(cplane)) = (id, cplane) {
            out.push((id, cplane));
        }
    }
    Ok(out)
}

/// `ON_MeshParameters::Read`: a version, then the switches, tolerances, edge lengths and grid
/// settings; from 1.1 the texture range, 1.2 custom settings and the relative tolerance, 1.3
/// the mesher, 1.4 whether custom settings are on (the SubD settings of 1.5 are not read).
fn read_mesh_settings(c: &mut Cur<'_>) -> R<MeshSettings> {
    let (major, minor) = c.version()?;
    if major != 1 {
        return super::err("mesh settings of an unknown version");
    }
    let mut m = MeshSettings { compute_curvature: c.i32()? != 0, simple_planes: c.i32()? != 0, refine: c.i32()? != 0, jagged_seams: c.i32()? != 0, ..MeshSettings::default() };
    c.i32()?; // obsolete weld
    m.tolerance = c.f64()?;
    m.min_edge_length = c.f64()?;
    m.max_edge_length = c.f64()?;
    m.grid_aspect_ratio = c.f64()?;
    m.grid_min_count = c.i32()?;
    m.grid_max_count = c.i32()?;
    m.grid_angle = c.f64()?;
    m.grid_amplification = c.f64()?;
    m.refine_angle = c.f64()?;
    c.f64()?; // obsolete combine angle
    m.face_type = c.u32()?;
    if minor >= 1 {
        m.texture_range = c.u32()?;
    }
    if minor >= 2 {
        m.custom = c.bool()?;
        m.relative_tolerance = c.f64()?;
    }
    if minor >= 3 {
        m.mesher = c.u8()?;
    }
    if minor >= 4 {
        m.custom_enabled = c.bool()?;
    }
    if minor >= 5 {
        c.chunk()?; // SubD display parameters
    }
    Ok(m)
}

/// `ON_3dmConstructionPlaneGridDefaults::Read`.
fn read_grid_defaults(mut c: Cur<'_>) -> R<GridDefaults> {
    let (major, _) = c.version()?;
    if major != 1 {
        return super::err("grid defaults of an unknown version");
    }
    Ok(GridDefaults {
        grid_spacing: c.f64()?,
        snap_spacing: c.f64()?,
        grid_line_count: c.i32()?,
        thick_frequency: c.i32()?,
        show_grid: c.i32()? != 0,
        show_axes: c.i32()? != 0,
        show_world_axes: c.i32()? != 0,
    })
}

/// `ON_3dmRenderSettings::Read`: in a chunk from Rhino 6 (files from November 2013 on), in
/// Rhino 5's layout (`ReadV5`, a version from 100) before.
fn read_render_settings(mut c: Cur<'_>) -> R<RenderSettings> {
    let mut r = RenderSettings::default();
    // Rhino 6 files before November 2013 have the old layout too: it starts with its version
    if c.v.archive <= 50 || c.peek_typecode() != Some(TCODE_ANONYMOUS_CHUNK) {
        let version = c.i32()?;
        if !(100..200).contains(&version) {
            return super::err("render settings of an unknown version");
        }
        r.custom_image_size = c.i32()? != 0;
        r.image_width = c.i32()?;
        r.image_height = c.i32()?;
        r.ambient = c.color()?;
        r.background_style = c.i32()?;
        r.background_color = c.color()?;
        r.background_bitmap = c.wstring()?;
        let mut flags = [false; 9];
        for f in &mut flags {
            *f = c.i32()? != 0;
        }
        [r.use_hidden_lights, r.depth_cue, r.flat_shade, r.render_backfaces, r.render_points, r.render_curves, r.render_isoparams, r.render_mesh_edges, r.render_annotations] = flags;
        r.antialias = c.i32()?;
        r.shadowmap = c.i32()?;
        r.shadowmap_width = c.i32()?;
        r.shadowmap_height = c.i32()?;
        r.shadowmap_offset = c.f64()?;
        if version >= 101 {
            r.image_dpi = c.f64()?;
            r.image_unit = c.u32()?;
        }
        if version >= 102 {
            r.background_bottom = c.color()?;
        }
        if version >= 103 {
            r.scale_background = c.bool()?;
        }
        return Ok(r);
    }
    let (mut b, major, minor) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    if major != 1 {
        return super::err("render settings of an unknown version");
    }
    r.custom_image_size = b.bool()?;
    r.image_width = b.i32()?;
    r.image_height = b.i32()?;
    r.image_dpi = b.f64()?;
    r.image_unit = b.u32()?;
    r.ambient = b.color()?;
    r.background_style = b.i32()?;
    r.background_color = b.color()?;
    r.background_bottom = b.color()?;
    r.background_bitmap = b.wstring()?;
    let mut flags = [false; 11];
    for f in &mut flags {
        *f = b.bool()?;
    }
    [r.use_hidden_lights, r.depth_cue, r.flat_shade, r.render_backfaces, r.render_points, r.render_curves, r.render_isoparams, r.render_mesh_edges, r.render_annotations, r.scale_background, r.transparent_background] = flags;
    r.antialias = b.i32()?;
    r.shadowmap = b.i32()?;
    r.shadowmap_width = b.i32()?;
    r.shadowmap_height = b.i32()?;
    r.shadowmap_offset = b.f64()?;
    if minor >= 1 {
        b.take(4 + 3 * 8 + 4)?; // obsolete
    }
    if minor >= 2 {
        r.rendering_source = b.i32()?;
        r.specific_viewport = b.wstring()?;
        r.named_view = b.wstring()?;
        r.snapshot = b.wstring()?;
    }
    if minor >= 3 {
        r.force_viewport_aspect_ratio = b.bool()?;
    }
    Ok(r)
}

/// `TCODE_SETTINGS_PLUGINLIST`: a version, a count and as many `ON_PlugInRef`.
fn read_plugins(mut c: Cur<'_>) -> R<Vec<PlugIn>> {
    let (major, _) = c.version()?;
    if major != 1 {
        return Ok(Vec::new());
    }
    let n = items(&mut c, 12)?;
    let mut out = Vec::with_capacity(n.min(1024));
    for _ in 0..n {
        let (mut b, major, minor) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
        if major != 1 {
            continue;
        }
        let mut p = PlugIn { id: b.uuid()?, kind: b.i32()?, name: b.wstring()?, version: b.wstring()?, filename: b.wstring()?, ..PlugIn::default() };
        if minor >= 1 {
            for field in [&mut p.organization, &mut p.address, &mut p.country, &mut p.phone, &mut p.email, &mut p.website, &mut p.update_url, &mut p.fax] {
                *field = b.wstring()?;
            }
        }
        if minor >= 2 {
            p.platform = b.i32()?;
            p.sdk_version = b.i32()?;
            p.sdk_service_release = b.i32()?;
        }
        out.push(p);
    }
    Ok(out)
}

/// `ON_UuidList::Read`: a chunk with a count and the ids.
fn uuid_list(c: &mut Cur<'_>) -> R<Vec<Uuid>> {
    let (mut b, _, _) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    let n = items(&mut b, 16)?;
    (0..n).map(|_| b.uuid()).collect()
}

/// `ON_HistoryRecord::Read` (the class data of a record of the history record table): its id,
/// version, command, descendants and antecedents, its values; from 1.1 its type, from 1.2
/// whether it is copied when its object is replaced.
pub(super) fn read_history_record(mut c: Cur<'_>) -> R<HistoryRecord> {
    let (mut b, major, minor) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    if major != 1 {
        return super::err("a history record of an unknown version");
    }
    let mut h = HistoryRecord { id: b.uuid()?, version: b.i32()?, command: b.uuid()?, ..HistoryRecord::default() };
    h.descendants = uuid_list(&mut b)?;
    h.antecedents = uuid_list(&mut b)?;
    let (mut values, _, _) = b.versioned(TCODE_ANONYMOUS_CHUNK)?;
    let n = items(&mut values, 12)?;
    for _ in 0..n {
        let (mut v, major, _) = values.versioned(TCODE_ANONYMOUS_CHUNK)?;
        if major != 1 {
            continue;
        }
        let kind = v.u32()?;
        let id = v.i32()?;
        h.values.push(HistoryValue { id, data: read_value(kind, &mut v)? });
    }
    if minor >= 1 {
        h.kind = b.i32()?;
    }
    if minor >= 2 {
        h.copy_on_replace = b.bool().unwrap_or(false);
    }
    Ok(h)
}

/// An `ON_Value`'s data (`ReadHelper`): an array of its type.
fn read_value(kind: u32, c: &mut Cur<'_>) -> R<HistoryData> {
    Ok(match kind {
        1 => {
            let n = c.count(1)?;
            HistoryData::Bools(c.take(n)?.iter().map(|&b| b != 0).collect())
        }
        2 => {
            let n = c.count(4)?;
            HistoryData::Ints((0..n).map(|_| c.i32()).collect::<R<_>>()?)
        }
        3 => {
            let n = c.count(8)?;
            HistoryData::Doubles((0..n).map(|_| c.f64()).collect::<R<_>>()?)
        }
        4 => {
            let n = c.count(4)?;
            HistoryData::Colors((0..n).map(|_| c.color()).collect::<R<_>>()?)
        }
        5 | 6 => {
            let n = c.count(24)?;
            let points = (0..n).map(|_| point(c)).collect::<R<_>>()?;
            if kind == 5 {
                HistoryData::Points(points)
            } else {
                HistoryData::Vectors(points)
            }
        }
        7 => {
            let n = c.count(128)?;
            HistoryData::Xforms((0..n).map(|_| c.xform()).collect::<R<_>>()?)
        }
        8 => {
            let n = c.count(4)?;
            HistoryData::Strings((0..n).map(|_| c.wstring()).collect::<R<_>>()?)
        }
        11 => {
            let n = c.count(16)?;
            HistoryData::Uuids((0..n).map(|_| c.uuid()).collect::<R<_>>()?)
        }
        9 => {
            // ON_ObjRef: a chunk each, the object's id first
            let n = c.count(12)?;
            let mut ids = Vec::with_capacity(n);
            for _ in 0..n {
                let (mut r, _, _) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
                ids.push(r.uuid()?);
            }
            HistoryData::Objects(ids)
        }
        _ => HistoryData::Raw(kind, c.take(c.left())?.to_vec()),
    })
}

/// A `TCODE_USER_TABLE`: the plug-in's id (with, from 2009, whether it was kept as goo and the
/// versions it was written in) and its record, as stored.
pub(super) fn read_user_table<'a>(mut c: Cur<'a>) -> R<UserTable<'a>> {
    let mut t = UserTable::default();
    while let Ok(chunk) = c.chunk() {
        match chunk.typecode {
            TCODE_ENDOFTABLE => break,
            TCODE_USER_TABLE_UUID => {
                let mut b = chunk.body;
                t.plugin = b.uuid()?;
                if b.peek_typecode() == Some(TCODE_USER_TABLE_RECORD_HEADER) {
                    let (mut h, _, _) = b.versioned(TCODE_USER_TABLE_RECORD_HEADER)?;
                    t.goo = h.bool()?;
                    t.archive_version = h.i32()?;
                    t.opennurbs_version = h.u32()?;
                }
            }
            TCODE_USER_RECORD => t.data = &chunk.body.data[chunk.body.pos..],
            _ => {}
        }
    }
    if t.plugin.is_nil() {
        return super::err("a user table without its plug-in id");
    }
    Ok(t)
}

/// What [`read_user_table`] found that is known: the document user text, the named layer
/// states and the snapshots, into `file`. `v` are the versions the table was written in.
pub(super) fn read_known_table(t: &UserTable<'_>, v: threedm_core::Versions, file: &mut File<'_>) {
    let c = Cur { data: t.data, pos: 0, v };
    if t.plugin == DOCUMENT_USER_STRINGS {
        if let Ok(Some(object)) = c.clone().object() {
            file.document_user_text.extend(threedm_core::user_strings_of(&object.userdata));
        }
    } else if t.plugin == LAYER_STATES {
        if let Ok(states) = read_layer_states(c) {
            file.layer_states = states;
        }
    } else if t.plugin == SNAPSHOTS {
        file.snapshots = snapshots(t.data);
    }
}

/// The table of Rhino's layer state manager: a version, a count, and of each state a version,
/// its name, the current layer's name, its layers (each a byte and an `ON_Layer` in a chunk,
/// then a chunk), from 1.2 the current layer's id.
fn read_layer_states(mut c: Cur<'_>) -> R<Vec<LayerState>> {
    let (major, _) = c.version()?;
    if major != 1 {
        return super::err("layer states of an unknown version");
    }
    let n = items(&mut c, 9)?;
    let mut out = Vec::with_capacity(n.min(1024));
    for _ in 0..n {
        let (major, minor) = c.version()?;
        if major != 1 {
            return super::err("a layer state of an unknown version");
        }
        let mut s = LayerState { name: c.wstring()?, current: c.wstring()?, ..LayerState::default() };
        let layers = items(&mut c, 1 + 24)?;
        for index in 0..layers as i32 {
            c.u8()?;
            let (mut b, _, _) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
            let record = b.chunk()?;
            if record.typecode == TCODE_LAYER_RECORD {
                if let Some(object) = record.body.clone().object()?.filter(|o| o.class == ON_LAYER) {
                    s.layers.push(read_layer(object.data, index)?);
                }
            }
            c.chunk()?;
        }
        if minor >= 2 {
            s.current_id = c.uuid()?;
        }
        out.push(s);
    }
    Ok(out)
}

/// The snapshots of the Snapshots plug-in's table: of each `<snap_shot>` element of its XML,
/// the name, the id and the element.
fn snapshots(data: &[u8]) -> Vec<Snapshot> {
    let Some(start) = data.windows(14).position(|w| w == b"<snapshot_data") else { return Vec::new() };
    let xml = String::from_utf8_lossy(&data[start..]);
    let mut out = Vec::new();
    let mut rest: &str = &xml;
    while let Some(i) = rest.find("<snap_shot ") {
        rest = &rest[i..];
        let end = rest.find("</snap_shot>").map(|e| e + "</snap_shot>".len()).unwrap_or(rest.len());
        let element = &rest[..end];
        let tag = &element[..element.find('>').unwrap_or(element.len())];
        let attribute = |name: &str| -> String {
            let key = format!(" {name}=\"");
            tag.find(&key).map(|k| &tag[k + key.len()..]).and_then(|v| v.find('"').map(|e| unescape(&v[..e]))).unwrap_or_default()
        };
        out.push(Snapshot { name: attribute("snap_shot_name"), id: attribute("snap_shot_id"), xml: element.to_string() });
        rest = &rest[end..];
    }
    out
}

/// XML's character entities.
fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&apos;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}
