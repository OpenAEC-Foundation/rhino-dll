//! The document's settings (`ON_3dmSettings`) and what else a file keeps beside its objects:
//! the earth anchor point and model base point, construction planes, mesh, grid and render
//! settings, the "current" table entries, the plug-ins, history records, fonts, dimension
//! styles and the tables plug-ins save (document user text, named layer states, snapshots).

use glam::{DMat4, DVec3};
use threedm_core::Uuid;

/// A font (`ON_Font`) as a file names it: of a dimension style, of a text (its first run).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Font {
    /// The family ("Arial", "Times New Roman").
    pub family: String,
    /// The face in the family as the file stores it ("Bold Italic", or localised as saved;
    /// empty when the file names only the family, as rich text does).
    pub face: String,
    pub postscript: String,
    /// The Windows LOGFONT face name (Rhino 4 and 5 store only this).
    pub logfont: String,
    /// `ON_Font::Weight`: 1 thin, 3 light, 4 normal, 5 medium, 6 semibold, 7 bold, 8
    /// ultrabold, 9 heavy (0 unset).
    pub weight: u8,
    /// `ON_Font::Style`: 1 upright, 2 italic, 3 oblique (0 unset).
    pub style: u8,
    pub bold: bool,
    pub italic: bool,
    pub underlined: bool,
    pub strikeout: bool,
}

/// A dimension style by id, index and name, with the font its texts and dimensions use.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DimensionStyle {
    pub id: Uuid,
    pub index: i32,
    pub name: String,
    pub font: Option<Font>,
}

/// The earth anchor point (`ON_EarthAnchorPoint`): where the model lies on earth, for
/// georeferencing.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EarthAnchor {
    /// Degrees north and east (WGS 84), and metres above the zero of `coordinate_system`.
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f64,
    /// `ON::EarthCoordinateSystem` of the elevation: 0 unset, 1 ground level, 2 mean sea level,
    /// 3 centre of the earth, 5 WGS 84, 6 EGM2008.
    pub coordinate_system: u8,
    /// The model point at that place, and the model's north and east there.
    pub model_point: DVec3,
    pub model_north: DVec3,
    pub model_east: DVec3,
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub url: String,
    pub url_tag: String,
}

/// A construction plane (`ON_3dmConstructionPlane`): named, or a view's.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConstructionPlane {
    pub name: String,
    pub origin: DVec3,
    pub x: DVec3,
    pub y: DVec3,
    pub z: DVec3,
    pub grid_spacing: f64,
    pub snap_spacing: f64,
    /// Grid lines on each side of the axes, and every how many lines one is thick.
    pub grid_line_count: i32,
    pub thick_frequency: i32,
    pub depth_buffered: bool,
}

/// The defaults of new construction planes' grids (`ON_3dmConstructionPlaneGridDefaults`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GridDefaults {
    pub grid_spacing: f64,
    pub snap_spacing: f64,
    pub grid_line_count: i32,
    pub thick_frequency: i32,
    pub show_grid: bool,
    pub show_axes: bool,
    pub show_world_axes: bool,
}

/// How surfaces are meshed (`ON_MeshParameters`): of render meshes, analysis meshes and the
/// document's custom render mesh settings.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MeshSettings {
    pub compute_curvature: bool,
    pub simple_planes: bool,
    pub refine: bool,
    pub jagged_seams: bool,
    pub tolerance: f64,
    pub min_edge_length: f64,
    pub max_edge_length: f64,
    pub grid_aspect_ratio: f64,
    pub grid_min_count: i32,
    pub grid_max_count: i32,
    /// Radians.
    pub grid_angle: f64,
    pub grid_amplification: f64,
    /// Radians.
    pub refine_angle: f64,
    pub face_type: u32,
    /// 1 packed and scaled to 0..1, 2 the surface's own (0 unset).
    pub texture_range: u32,
    pub custom: bool,
    /// The "density" slider (0 to 1) of Rhino's simple mesh settings.
    pub relative_tolerance: f64,
    pub mesher: u8,
    pub custom_enabled: bool,
}

/// The render settings of the document (`ON_3dmRenderSettings`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DocumentRenderSettings {
    pub custom_image_size: bool,
    pub image_width: i32,
    pub image_height: i32,
    pub image_dpi: f64,
    /// `ON::LengthUnitSystem` of the image size.
    pub image_unit: u32,
    pub ambient: [u8; 4],
    /// 0 solid colour, 1 gradient, 2 wallpaper, 3 environment.
    pub background_style: i32,
    pub background_color: [u8; 4],
    pub background_bottom: [u8; 4],
    pub background_bitmap: String,
    pub use_hidden_lights: bool,
    pub depth_cue: bool,
    pub flat_shade: bool,
    pub render_backfaces: bool,
    pub render_points: bool,
    pub render_curves: bool,
    pub render_isoparams: bool,
    pub render_mesh_edges: bool,
    pub render_annotations: bool,
    pub scale_background: bool,
    pub transparent_background: bool,
    pub antialias: i32,
    pub shadowmap: i32,
    pub shadowmap_width: i32,
    pub shadowmap_height: i32,
    pub shadowmap_offset: f64,
    /// 0 the active viewport, 1 a viewport, 2 a named view, 3 a snapshot.
    pub rendering_source: i32,
    pub specific_viewport: String,
    pub named_view: String,
    pub snapshot: String,
    pub force_viewport_aspect_ratio: bool,
}

/// The "current" entries of the document's tables: what new objects get. Rhino 5 files name
/// them by index, later files by id; what a file does not say is `None` or nil.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Current {
    pub layer_index: Option<i32>,
    pub layer: Uuid,
    pub material_index: Option<i32>,
    pub material: Uuid,
    pub material_source: i32,
    pub color: [u8; 4],
    pub color_source: i32,
    pub plot_color: [u8; 4],
    pub plot_color_source: i32,
    pub linetype_index: Option<i32>,
    pub linetype: Uuid,
    pub linetype_source: i32,
    pub text_style_index: Option<i32>,
    pub text_style: Uuid,
    pub dimension_style_index: Option<i32>,
    pub dimension_style: Uuid,
    pub hatch_pattern: Uuid,
    /// Isocurves of new surfaces (-1 none).
    pub wire_density: i32,
}

/// A plug-in the file was saved with (`ON_PlugInRef`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlugIn {
    pub id: Uuid,
    /// `ON::plugin_type`: 1 render, 2 file import, 3 file export, 4 digitiser, 5 utility, 6
    /// display pipeline, 7 display engine.
    pub kind: i32,
    pub name: String,
    pub version: String,
    pub filename: String,
    pub organization: String,
    pub address: String,
    pub country: String,
    pub phone: String,
    pub email: String,
    pub website: String,
    pub update_url: String,
    pub fax: String,
    /// 0 native, 1 .NET.
    pub platform: i32,
    pub sdk_version: i32,
    pub sdk_service_release: i32,
}

/// The document settings (`ON_3dmSettings`) besides the units and views, which [`crate::File`]
/// holds itself.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings {
    pub earth_anchor: Option<EarthAnchor>,
    /// The model's base point (`m_model_basepoint`), the origin of its insertions elsewhere.
    pub model_basepoint: DVec3,
    pub model_url: String,
    pub named_cplanes: Vec<ConstructionPlane>,
    /// The construction plane of each view of the view list, by viewport id.
    pub view_cplanes: Vec<(Uuid, ConstructionPlane)>,
    pub render_mesh: Option<MeshSettings>,
    pub analysis_mesh: Option<MeshSettings>,
    pub custom_render_mesh: Option<MeshSettings>,
    pub grid: Option<GridDefaults>,
    pub render: Option<DocumentRenderSettings>,
    pub current: Current,
    pub linetype_display_scale: f64,
    pub active_view: Uuid,
    pub save_texture_bitmaps: bool,
    /// How linked block definitions are updated: 1 never, 2 prompt, 3 always (0 unset).
    pub idef_link_update: i32,
    pub plugins: Vec<PlugIn>,
}

/// A value of a history record (`ON_Value`): the id the command gives it and what it holds.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryValue {
    pub id: i32,
    pub data: HistoryData,
}

/// What an `ON_Value` holds, by its type.
#[derive(Debug, Clone, PartialEq)]
pub enum HistoryData {
    Bools(Vec<bool>),
    Ints(Vec<i32>),
    Doubles(Vec<f64>),
    Colors(Vec<[u8; 4]>),
    Points(Vec<DVec3>),
    Vectors(Vec<DVec3>),
    Xforms(Vec<DMat4>),
    Strings(Vec<String>),
    Uuids(Vec<Uuid>),
    /// Object references: the ids of the objects.
    Objects(Vec<Uuid>),
    /// Geometry, points on objects, polyedges, SubD edge chains: the type (`ON_Value`'s
    /// `VALUE_TYPE`) and the bytes as stored.
    Raw(u32, Vec<u8>),
}

impl HistoryData {
    /// The type's name.
    pub fn kind(&self) -> &'static str {
        match self {
            HistoryData::Bools(_) => "bool",
            HistoryData::Ints(_) => "int",
            HistoryData::Doubles(_) => "double",
            HistoryData::Colors(_) => "color",
            HistoryData::Points(_) => "point",
            HistoryData::Vectors(_) => "vector",
            HistoryData::Xforms(_) => "xform",
            HistoryData::Strings(_) => "string",
            HistoryData::Uuids(_) => "uuid",
            HistoryData::Objects(_) => "objref",
            HistoryData::Raw(10, _) => "geometry",
            HistoryData::Raw(12, _) => "point on object",
            HistoryData::Raw(13, _) => "polyedge",
            HistoryData::Raw(14, _) => "subd edge chain",
            HistoryData::Raw(..) => "unknown",
        }
    }
}

/// A history record (`ON_HistoryRecord`): the command that made objects, from which objects,
/// with which values, so that it can update them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HistoryRecord {
    pub id: Uuid,
    /// The command's id and its history version.
    pub command: Uuid,
    pub version: i32,
    /// 0 history parameters, 1 feature parameters.
    pub kind: i32,
    /// The objects it was made from, and the objects it made.
    pub antecedents: Vec<Uuid>,
    pub descendants: Vec<Uuid>,
    pub copy_on_replace: bool,
    pub values: Vec<HistoryValue>,
}

/// A table a plug-in saved in the file (`TCODE_USER_TABLE`), as stored.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UserTable<'a> {
    pub plugin: Uuid,
    /// The plug-in's name, from the file's plug-in list.
    pub name: String,
    /// Kept unread by an application that did not have the plug-in ("goo").
    pub goo: bool,
    /// The archive and openNURBS versions it was written in.
    pub archive_version: i32,
    pub opennurbs_version: u32,
    pub data: &'a [u8],
}

/// A named layer state (Rhino's layer state manager): the layers as they were saved.
#[derive(Debug, Clone, Default)]
pub struct LayerState {
    pub name: String,
    /// The current layer then: its name and id.
    pub current: String,
    pub current_id: Uuid,
    pub layers: Vec<crate::Layer>,
}

/// A snapshot (Rhino's Snapshots plug-in): its name and id and its content as the plug-in
/// saved it (XML).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Snapshot {
    pub name: String,
    pub id: String,
    pub xml: String,
}
