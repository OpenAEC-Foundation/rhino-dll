//! The neutral model the 3dm reader produces: the layers, materials, bitmaps and block
//! definitions of a Rhino file, and its objects with their attributes, user text and geometry,
//! as meshes, polylines, points or block instances. Exact geometry (polysurfaces, surfaces,
//! extrusions, SubD, curves, texts and hatches) is turned into these by the reader.

use glam::{DMat4, DVec3};
use std::collections::BTreeMap;
pub use threedm_core::{Uuid, Versions};

mod settings;
pub use settings::*;
mod render;
pub use render::*;

/// A layer (`ON_Layer`); its full path follows the parents by id.
#[derive(Debug, Clone, Default)]
pub struct Layer {
    /// The index objects refer to.
    pub index: i32,
    pub name: String,
    pub id: Uuid,
    pub parent: Uuid,
    pub color: [u8; 4],
    pub visible: bool,
    pub material: i32,
    /// The index of its linetype (`-1` continuous).
    pub linetype: i32,
    /// Print colour (unset in the file: its colour) and width (millimetres; 0 the default,
    /// negative no print).
    pub plot_color: [u8; 4],
    pub plot_weight: f64,
    /// The section style of the section style table its objects are cut with (Rhino 9), by
    /// index; `-1` none.
    pub section_style: i32,
    /// Its own section style (Rhino 8), over the table's.
    pub custom_section_style: Option<SectionStyle>,
}

/// How the ends of a wide line are drawn (`ON::LineCapStyle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineCap {
    #[default]
    Round,
    Flat,
    Square,
}

/// How the corners of a wide line are drawn (`ON::LineJoinStyle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineJoin {
    #[default]
    Round,
    Miter,
    Bevel,
}

/// A linetype (`ON_Linetype`) of the linetype table, or an object's own.
#[derive(Debug, Clone, PartialEq)]
pub struct Linetype {
    /// The index objects and layers refer to.
    pub index: i32,
    pub id: Uuid,
    pub name: String,
    /// Dashes as positive lengths and gaps as negative ones.
    pub segments: Vec<f64>,
    /// Rhino 8: how wide its lines are drawn, in `width_unit` (`ON::LengthUnitSystem`: 0 none,
    /// 2 millimetres, 3 centimetres, 4 metres, 8 inches, ..).
    pub width: f64,
    pub width_unit: u8,
    pub cap: LineCap,
    pub join: LineJoin,
    /// A tapered width: points (position along the curve from 0 to 1, width).
    pub taper: Vec<[f64; 2]>,
    /// Its pattern is in model distances also on layouts and in details.
    pub always_model_distances: bool,
}

impl Default for Linetype {
    fn default() -> Self {
        Linetype { index: -1, id: Uuid::default(), name: String::new(), segments: Vec::new(), width: 1.0, width_unit: 0, cap: LineCap::Round, join: LineJoin::Round, taper: Vec::new(), always_model_distances: false }
    }
}

/// How a curve's line is drawn wide: its linetype's width, ends, corners and taper, and its
/// print width and colour (from the object or its layer).
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    /// The linetype it comes from by index; `-1` the object's own linetype or continuous.
    pub linetype: i32,
    /// The width as the linetype gives it, in `width_unit` (`ON::LengthUnitSystem`, 0 none).
    pub width: f64,
    pub width_unit: u8,
    /// The width in model units, when `width_unit` is a length.
    pub model_width: Option<f64>,
    pub cap: LineCap,
    pub join: LineJoin,
    /// Points (position along the curve from 0 to 1, width in `width_unit`).
    pub taper: Vec<[f64; 2]>,
    /// Print width in millimetres (0 the default, negative no print) and print colour.
    pub print_width: f64,
    pub print_color: [u8; 4],
}

/// A section style (`ON_SectionStyle`): how the cut of an object by a clipping plane is drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionStyle {
    /// Its index in the section style table (Rhino 9); `-1` an object's or layer's own.
    pub index: i32,
    pub id: Uuid,
    pub name: String,
    /// 0 no background fill, 1 as the viewport draws the object, 2 a solid colour.
    pub background_fill_mode: u8,
    /// Colours as `ON_Color` (alpha 0 opaque); [`UNSET_COLOR`]: the object's own.
    pub background_fill_color: [u8; 4],
    pub background_fill_print_color: [u8; 4],
    pub boundary_visible: bool,
    pub boundary_color: [u8; 4],
    pub boundary_print_color: [u8; 4],
    /// The boundary's width as a multiple of the object's.
    pub boundary_width_scale: f64,
    /// 0 closed curves are filled, 1 only the cuts of solid objects.
    pub fill_rule: u8,
    /// The hatch pattern (index) the cut is hatched with, [`UNSET_INDEX`] none; its scale,
    /// rotation (radians) and colours.
    pub hatch_index: i32,
    pub hatch_scale: f64,
    pub hatch_rotation: f64,
    pub hatch_color: [u8; 4],
    pub hatch_print_color: [u8; 4],
    /// The boundary's own linetype, when it has one.
    pub boundary_linetype: Option<Linetype>,
    /// Rhino 9: the boundary's linetype by index ([`UNSET_INDEX`] none), and the print widths
    /// (millimetres, -10 unset) of boundary and hatch.
    pub boundary_linetype_index: i32,
    pub boundary_print_width: f64,
    pub hatch_print_width: f64,
}

/// `ON_Color::UnsetColor`.
pub const UNSET_COLOR: [u8; 4] = [255; 4];
/// `ON_UNSET_INT_INDEX`.
pub const UNSET_INDEX: i32 = i32::MIN + 1;

impl Default for SectionStyle {
    fn default() -> Self {
        SectionStyle {
            index: -1,
            id: Uuid::default(),
            name: String::new(),
            background_fill_mode: 1,
            background_fill_color: UNSET_COLOR,
            background_fill_print_color: UNSET_COLOR,
            boundary_visible: true,
            boundary_color: UNSET_COLOR,
            boundary_print_color: UNSET_COLOR,
            boundary_width_scale: 3.0,
            fill_rule: 0,
            hatch_index: UNSET_INDEX,
            hatch_scale: 1.0,
            hatch_rotation: 0.0,
            hatch_color: UNSET_COLOR,
            hatch_print_color: UNSET_COLOR,
            boundary_linetype: None,
            boundary_linetype_index: UNSET_INDEX,
            boundary_print_width: -10.0,
            hatch_print_width: -10.0,
        }
    }
}

/// A material (`ON_Material`): its colours, shine, reflectivity, index of refraction and
/// transparency, its textures, and its physically based parameters when it has them.
#[derive(Debug, Clone, Default)]
pub struct Material {
    pub index: i32,
    pub name: String,
    pub id: Uuid,
    /// The plug-in it belongs to (the RDK's for materials made by render content).
    pub plug_in: Uuid,
    /// The render content it simulates ([`RenderContent::id`]), nil if none.
    pub render_material: Uuid,
    pub ambient: [u8; 4],
    pub diffuse: [u8; 4],
    pub emission: [u8; 4],
    pub specular: [u8; 4],
    pub reflection: [u8; 4],
    /// The colour of transparent parts.
    pub transparent: [u8; 4],
    /// 0 to 255 (`ON_Material::MaxShine`).
    pub shine: f64,
    /// 0 to 1.
    pub reflectivity: f64,
    pub index_of_refraction: f64,
    /// 0 opaque to 1 fully transparent.
    pub transparency: f64,
    pub textures: Vec<Texture>,
    /// `ON_PhysicallyBasedMaterial`, when the material is physically based.
    pub pbr: Option<Pbr>,
}

/// A texture of a material (`ON_Texture`).
#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    /// The image file as stored (a full or a relative path).
    pub file: String,
    /// `ON_Texture::TYPE`: 1 bitmap (diffuse, PBR base colour), 2 bump, 3 transparency
    /// (opacity), 10 to 30 the other PBR slots, 86 environment.
    pub kind: u32,
    pub enabled: bool,
    /// `ON_Texture::MODE`: 0 none, 1 modulate, 2 decal, 3 blend.
    pub mode: u32,
    /// The blend constant (`m_blend_constant_A`), 1 the texture alone.
    pub amount: f64,
    /// The transform of the texture coordinates (`m_uvw`), for column vectors.
    pub uvw: DMat4,
    /// The mapping channel the coordinates come from.
    pub mapping_channel: i32,
}

impl Default for Texture {
    fn default() -> Self {
        Texture { file: String::new(), kind: 1, enabled: true, mode: 1, amount: 1.0, uvw: DMat4::IDENTITY, mapping_channel: 1 }
    }
}

/// The physically based parameters of a material (`ON_PhysicallyBasedMaterial`), colours as
/// linear red, green, blue and alpha floats.
#[derive(Debug, Clone, PartialEq)]
pub struct Pbr {
    pub base_color: [f32; 4],
    /// `ON_PhysicallyBasedMaterial::BRDFs`: 0 GGX, 1 Lambertian.
    pub brdf: i32,
    pub subsurface: f64,
    pub subsurface_scattering_color: [f32; 4],
    pub subsurface_scattering_radius: f64,
    pub metallic: f64,
    pub specular: f64,
    pub specular_tint: f64,
    pub roughness: f64,
    pub anisotropic: f64,
    pub anisotropic_rotation: f64,
    pub sheen: f64,
    pub sheen_tint: f64,
    pub clearcoat: f64,
    pub clearcoat_roughness: f64,
    pub opacity_ior: f64,
    pub opacity: f64,
    pub opacity_roughness: f64,
    pub emission: [f32; 4],
    /// 1 unless set (chunk version 2).
    pub alpha: f64,
}

/// An image of the bitmap table (`ON_EmbeddedBitmap`, or the file name only of an
/// `ON_WindowsBitmapEx`).
#[derive(Debug, Clone, Default)]
pub struct Bitmap {
    pub name: String,
    /// The image file as it was named when it was embedded.
    pub file: String,
    /// The file's bytes, when embedded.
    pub data: Option<Vec<u8>>,
}

/// A block definition (`ON_InstanceDefinition`): the ids of its member objects.
#[derive(Debug, Clone)]
pub struct BlockDefinition {
    pub id: Uuid,
    pub name: String,
    pub objects: Vec<Uuid>,
}

/// `ON_3dmObjectAttributes`, the parts read.
#[derive(Debug, Clone)]
pub struct Attributes {
    pub id: Uuid,
    pub name: String,
    pub layer: i32,
    pub material: i32,
    pub color: [u8; 4],
    /// 0 by layer, 1 by object, 2 by material, 3 by parent.
    pub color_source: u8,
    /// 0 by layer, 1 by object, 3 by parent.
    pub material_source: u8,
    pub visible: bool,
    /// 0 normal, 1 hidden, 2 locked, 3 block definition geometry.
    pub mode: u8,
    /// 1 model space, 2 page (layout) space.
    pub space: u8,
    /// The linetype by index, and where it comes from: 0 by layer, 1 by object, 3 by parent.
    pub linetype: i32,
    pub linetype_source: u8,
    /// The object's own linetype (Rhino 8), dashes as positive lengths and gaps as negative.
    pub custom_linetype: Option<Vec<f64>>,
    /// The scale of its linetype's pattern.
    pub linetype_scale: f64,
    pub url: String,
    /// Print colour and width (millimetres; 0 the default, negative no print) and where they
    /// come from (0 by layer, 1 by object, 2 by display colour, 3 by parent).
    pub plot_color: [u8; 4],
    pub plot_weight: f64,
    pub plot_color_source: u8,
    pub plot_weight_source: u8,
    /// The groups it is in, by index into [`File::groups`].
    pub groups: Vec<i32>,
    /// When not nil: the only viewport it shows in.
    pub viewport: Uuid,
    /// Its place in the draw order (higher on top) of drafting.
    pub display_order: i32,
    /// A hatch's background fill colour, and whether its boundary is drawn (Rhino 8).
    pub hatch_background: Option<[u8; 4]>,
    pub hatch_boundary: bool,
    /// The object's own linetype (Rhino 8) in full: with its width, cap, join and taper.
    pub custom_linetype_style: Option<Linetype>,
    /// Where its section style comes from: 0 by layer, 1 by object, 2 by parent, 3 from the
    /// clipping plane that cuts it.
    pub section_source: u8,
    /// Its section style of the table (Rhino 9) by index, `-1` none, and its own (Rhino 8).
    pub section_style: i32,
    pub custom_section_style: Option<SectionStyle>,
    /// A clipping plane's label: 0 none, 1 a text dot of its name, 2 a text of its name.
    pub clipping_label: u8,
    /// Its decals.
    pub decals: Vec<Decal>,
    /// Its mesh modifiers (only read: its meshes are as stored).
    pub mesh_modifiers: MeshModifiers,
}

impl Default for Attributes {
    fn default() -> Self {
        Attributes {
            id: Uuid::default(),
            name: String::new(),
            layer: 0,
            material: -1,
            color: [0, 0, 0, 0],
            color_source: 0,
            material_source: 0,
            visible: true,
            mode: 0,
            space: 1,
            linetype: -1,
            linetype_source: 0,
            custom_linetype: None,
            linetype_scale: 1.0,
            url: String::new(),
            plot_color: [0, 0, 0, 0],
            plot_weight: 0.0,
            plot_color_source: 0,
            plot_weight_source: 0,
            groups: Vec::new(),
            viewport: Uuid::default(),
            display_order: 0,
            hatch_background: None,
            hatch_boundary: false,
            custom_linetype_style: None,
            section_source: 0,
            section_style: -1,
            custom_section_style: None,
            clipping_label: 0,
            decals: Vec::new(),
            mesh_modifiers: MeshModifiers::default(),
        }
    }
}

/// A triangle mesh as stored in an `ON_Mesh`: float vertices, optional normals and colours,
/// faces of four indices (a triangle repeats its third).
#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Option<Vec<[f32; 3]>>,
    pub colors: Option<Vec<[u8; 4]>>,
    pub faces: Vec<[u32; 4]>,
    /// The triangles of the faces (a quad is two).
    pub triangles: usize,
    /// Texture coordinates (`m_T`), one per vertex.
    pub texture_coordinates: Option<Vec<[f32; 2]>>,
    /// The colour of the Brep face this mesh belongs to (Rhino 7 and later), over the object's.
    pub color: Option<[u8; 4]>,
    /// Made by the reader from the exact geometry: the file has no mesh for it.
    pub computed: bool,
}

#[derive(Debug, Clone)]
pub enum Geometry<'a> {
    /// Meshes and the object type to show ("Mesh", "Polysurface", ..).
    Meshes(Vec<Mesh>, &'static str),
    Instance { definition: Uuid, xform: DMat4 },
    /// Points as stored, three doubles (24 bytes) each, and their colours (4 bytes each): read
    /// only while drawing, so that a large cloud is thinned out without a copy of every point.
    Points { points: &'a [u8], colors: Option<&'a [u8]>, kind: &'static str },
    /// Polylines (curves, and the lines of text and hatches) and the object type to show.
    Lines(Vec<Vec<DVec3>>, &'static str),
    /// Lines and filled shapes together, and the object type to show: dimensions and leaders
    /// (their lines; their arrowheads and text as meshes).
    Drawing { lines: Vec<Vec<DVec3>>, meshes: Vec<Mesh>, kind: &'static str },
    /// A clipping plane: the rectangle Rhino draws for it and the plane it cuts with.
    ClippingPlane(ClippingPlane),
    /// A detail of a layout (`ON_DetailView`): a window on the page that shows the model.
    Detail {
        /// Its outline on the page (page units), as stored: a closed polyline, the first
        /// corner repeated at the end.
        boundary: Vec<DVec3>,
        /// The model view it shows: its camera and projection.
        view: View,
        /// Page length per model length (`m_page_per_model_ratio`), when set: a detail at
        /// 1:100 with page and model in millimetres has 0.01. `None` for a perspective.
        scale: Option<f64>,
        /// Its projection is locked (Rhino's Lock detail).
        locked: bool,
    },
    /// Not drawn: why, for the count in the project information.
    Skipped(&'static str),
}

/// A clipping plane (`ON_ClippingPlaneSurface`). It cuts away what lies on the side its
/// `normal` points to, in the viewports it names, when it is enabled.
#[derive(Debug, Clone, PartialEq)]
pub struct ClippingPlane {
    /// The rectangle Rhino draws for it, corner after corner.
    pub corners: [DVec3; 4],
    pub origin: DVec3,
    /// Unit normal, to the side that is cut away.
    pub normal: DVec3,
    pub enabled: bool,
    /// The viewports it clips (their ids, as in [`View::id`]).
    pub viewports: Vec<Uuid>,
    /// When it clips only a slab: how deep behind the plane (along −`normal`) it clips.
    pub depth: Option<f64>,
    /// When it clips only some objects (Rhino 8's selective clipping): which.
    pub selection: Option<ClipSelection>,
}

/// The objects a clipping plane clips: these objects and the objects on these layers, or, when
/// `except`, all others.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ClipSelection {
    pub objects: Vec<Uuid>,
    /// Layer indices.
    pub layers: Vec<i32>,
    pub except: bool,
}

/// A viewport of the file's views (`ON_3dmView`); the first is the view that was active. The
/// views of layouts are among them, with their [`Page`].
#[derive(Debug, Clone, PartialEq)]
pub struct View {
    pub id: Uuid,
    pub name: String,
    pub perspective: bool,
    /// The camera: its location, unit direction and up (model units).
    pub location: DVec3,
    pub direction: DVec3,
    pub up: DVec3,
    /// Vertical field of view (radians) of a perspective view; the height it shows (model
    /// units) of a parallel one.
    pub fov_y: f64,
    pub height: f64,
    /// A layout (page view, `ON::page_view_type`): its paper; `None` for a model view.
    pub page: Option<Page>,
}

/// The paper of a layout (`ON_3dmPageSettings`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Page {
    /// Width and height in millimetres.
    pub width: f64,
    pub height: f64,
    /// Its place among the layouts, from 0.
    pub number: i32,
}

/// What kind of light a [`Light`] is (`ON::light_style`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightKind {
    /// Light from one direction, as of the sun.
    Directional,
    /// Light from a point in all directions.
    Point,
    /// Light from a point in a cone.
    Spot,
    /// Light from a line: from `location` along `length`.
    Linear,
    /// Light from a rectangle: at `location`, spanned by `length` and `width`.
    Rectangular,
    /// Light everywhere, from no direction.
    Ambient,
    /// A style openNURBS does not know.
    Unknown,
}

/// A light (`ON_Light`) of the light table, with the layer and visibility of its attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct Light {
    pub id: Uuid,
    pub name: String,
    pub layer: i32,
    pub visible: bool,
    /// Switched on.
    pub on: bool,
    pub kind: LightKind,
    /// Placed in camera coordinates (+x right, +y up, +z towards the camera) and moving with
    /// the camera, not in world coordinates.
    pub camera: bool,
    /// The `ON::light_style` as stored.
    pub style: i32,
    pub location: DVec3,
    /// Where it shines (not unit: a spot light's cone reaches as far).
    pub direction: DVec3,
    /// The sides of a linear or rectangular light.
    pub length: DVec3,
    pub width: DVec3,
    pub ambient: [u8; 4],
    pub diffuse: [u8; 4],
    pub specular: [u8; 4],
    /// 0 to 1.
    pub intensity: f64,
    pub watts: f64,
    /// A spot light's half angle in degrees (0 to 90), its exponent and its hot spot (0 to 1),
    /// as Rhino gives them (a light keeps one of the last two, the other follows from it).
    pub spot_angle: f64,
    pub spot_exponent: f64,
    pub hotspot: f64,
    /// Constant, linear and quadratic attenuation: the light falls off as 1/(a + b·d + c·d²).
    pub attenuation: DVec3,
    pub shadow_intensity: f64,
}

/// An object of the object table: its geometry, attributes and user text.
#[derive(Debug, Clone)]
pub struct Object<'a> {
    pub geometry: Geometry<'a>,
    pub attributes: Attributes,
    pub user: Vec<(String, String)>,
    /// Of a curve: how its line is drawn wide (its linetype's width, cap and join, its print
    /// width).
    pub stroke: Option<Stroke>,
    /// The font of a text, dimension or leader: of its first character as Rhino gives it.
    pub font: Option<Font>,
}

/// A Rhino file as read.
#[derive(Debug, Default)]
pub struct File<'a> {
    pub versions: Versions,
    pub application: String,
    /// Metres per model unit, and the unit's name.
    pub meters: f64,
    pub unit: &'static str,
    /// Metres per unit of the layouts (page space), and its name: the model's unless the file
    /// says otherwise.
    pub page_meters: f64,
    pub page_unit: &'static str,
    pub layers: Vec<Layer>,
    /// The linetype table.
    pub linetypes: Vec<Linetype>,
    /// The section style table (Rhino 9).
    pub section_styles: Vec<SectionStyle>,
    pub materials: Vec<Material>,
    /// The bitmap table: images embedded in the file.
    pub bitmaps: Vec<Bitmap>,
    pub blocks: Vec<BlockDefinition>,
    pub objects: Vec<Object<'a>>,
    /// The viewports of the file's views, the active one first.
    pub views: Vec<View>,
    /// The views saved by name (`NamedView`).
    pub named_views: Vec<View>,
    /// The group table: of each group its index and name.
    pub groups: Vec<(i32, String)>,
    /// The lights of the light table (and of the object table, where old files keep them).
    pub lights: Vec<Light>,
    /// The document settings: earth anchor, construction planes, mesh, grid and render
    /// settings, the current table entries, the plug-ins.
    pub settings: Settings,
    /// The dimension styles with their fonts.
    pub dim_styles: Vec<DimensionStyle>,
    /// The font table of Rhino 4 and 5 files (`ON_TextStyle`), by index.
    pub fonts: Vec<(i32, Font)>,
    /// The history records (`ON_HistoryRecord`).
    pub history: Vec<HistoryRecord>,
    /// The tables plug-ins saved, as stored; those read below are among them.
    pub user_tables: Vec<UserTable<'a>>,
    /// The document's user text (Rhino's `SetDocumentUserText`), key and value.
    pub document_user_text: Vec<(String, String)>,
    /// The named layer states, and the snapshots.
    pub layer_states: Vec<LayerState>,
    pub snapshots: Vec<Snapshot>,
    /// The render settings, with the rendering parts of the RDK document.
    pub render: RenderSettings,
    /// The render content of the RDK document: materials, environments and textures, each
    /// followed by the content in its child slots.
    pub render_content: Vec<RenderContent>,
    /// Records that could not be read, by table.
    pub damaged: BTreeMap<&'static str, usize>,
    /// The user data of objects and table records, every entry as stored.
    pub plugin_data: Vec<PlugInData<'a>>,
    /// The NURBS volumes of cage objects (`ON_NurbsCage`), by their object's place in
    /// [`File::objects`].
    pub cages: Vec<(usize, NurbsCage)>,
    /// The morph controls (`ON_MorphControl`).
    pub morph_controls: Vec<MorphControl>,
    /// The segments of curves along polysurface edges (`ON_PolyEdgeCurve`), by their object's
    /// place in [`File::objects`].
    pub poly_edges: Vec<(usize, Vec<PolyEdgeSegment>)>,
}

/// Who a piece of plug-in data belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    /// An object, by its place in [`File::objects`]: the data of its geometry, or (`attributes`)
    /// of its attributes.
    Object { index: usize, attributes: bool },
    /// A record of a table ("layers", "materials", ..), by its place in the table.
    Table { table: &'static str, index: usize },
}

/// A user data entry (`ON_UserData`) as stored, with what is known of it: the plug-in that
/// wrote it (named when the file's plug-in list, [`Settings::plugins`], names it) and, when its form is known, its
/// content ([`Decoded`]).
#[derive(Debug, Clone)]
pub struct PlugInData<'a> {
    pub owner: Owner,
    /// The user data class.
    pub class: Uuid,
    /// The plug-in (`m_application_uuid`); nil in old files.
    pub plugin: Uuid,
    pub plugin_name: String,
    /// `m_userdata_uuid`: which item of the plug-in's data.
    pub item: Uuid,
    /// The version of the entry (header 1.0 to 2.2).
    pub version: (u8, u8),
    pub copy_count: i32,
    /// The transforms applied to the owner since the data was attached, for column vectors.
    pub xform: DMat4,
    /// Saved by a program that did not know its class (`ON_UnknownUserData`).
    pub goo: bool,
    /// The archive and openNURBS versions it was written with (0 when the file does not say).
    pub archive: i32,
    pub opennurbs: u32,
    /// The bytes as stored (the content of its anonymous chunk).
    pub bytes: &'a [u8],
    pub decoded: Option<Decoded>,
}

/// The content of plug-in data whose form is known.
#[derive(Debug, Clone, PartialEq)]
pub enum Decoded {
    /// `ON_UserStringList` (user text) or the document's user text.
    UserText(Vec<(String, String)>),
    /// An archivable dictionary (RhinoCommon's `UserDictionary`, `ON_ArchivableDictionary`).
    Dictionary(Dictionary),
    /// Text the data holds whole: XML or JSON (UTF-8 or UTF-16).
    Text(String),
}

/// An archivable dictionary (`ON_ArchivableDictionary`, RhinoCommon's `ArchivableDictionary`):
/// its id, version and name, and its entries in the order stored.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Dictionary {
    pub id: Uuid,
    pub version: u32,
    pub name: String,
    pub entries: Vec<Entry>,
}

/// An entry of a dictionary: its key, its type as stored and its value.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub key: String,
    /// The item type as stored (`DictionaryEntryType`: 1 bool, 6 int, 10 double, 12 string, ..).
    pub kind: i32,
    pub value: Value,
}

/// A value of a dictionary entry.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    /// Every integer type (8 to 64 bits, signed or not).
    Int(i64),
    /// Float and double.
    Number(f64),
    Uuid(Uuid),
    String(String),
    /// Red, green, blue and alpha (0 opaque), as `ON_Color` stores it.
    Color([u8; 4]),
    /// Points, vectors, intervals, rectangles, sizes, boxes, rays, planes, lines and
    /// transforms: their numbers in the order stored (see [`Entry::kind`]).
    Numbers(Vec<f64>),
    /// Arrays of bools, integers, floats, doubles, uuids and strings.
    Array(Vec<Value>),
    Dictionary(Box<Dictionary>),
    /// An openNURBS object (geometry, mesh parameters): its class and data length.
    Object { class: Uuid, bytes: usize },
    /// A type the reader does not know, or could not read: the bytes as stored.
    Raw(Vec<u8>),
}

/// A NURBS volume (`ON_NurbsCage`), or a NURBS curve or surface as one with a single control
/// point in the directions it lacks.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NurbsCage {
    /// Order and control point count in each of the three directions.
    pub order: [usize; 3],
    pub count: [usize; 3],
    /// The knots of each direction as openNURBS keeps them (order + count − 2).
    pub knots: [Vec<f64>; 3],
    /// Homogeneous control points (x·w, y·w, z·w, w), the third direction fastest.
    pub points: Vec<[f64; 4]>,
}

/// A morph control (`ON_MorphControl`): the curve, surface or cage that deforms its captive
/// objects.
#[derive(Debug, Clone, PartialEq)]
pub struct MorphControl {
    /// Its object, by its place in [`File::objects`].
    pub object: usize,
    /// `m_varient`: 1 curve, 2 surface, 3 cage.
    pub kind: i32,
    /// Where the control deforms to.
    pub control: NurbsCage,
    /// Where a curve or surface control starts from.
    pub start: Option<NurbsCage>,
    /// Of a cage: the transform from world to the cage's parameters at the start.
    pub cage_xform: Option<DMat4>,
    /// The objects it deforms.
    pub captives: Vec<Uuid>,
    /// The number of its localizers (the regions it acts in).
    pub localizers: usize,
    /// Space morph tolerance, quick preview and preserve structure (version 2.1).
    pub tolerance: f64,
    pub quick_preview: bool,
    pub preserve_structure: bool,
}

/// A segment of a curve along polysurface edges (`ON_PolyEdgeSegment`), as stored: the object
/// and component (edge or trim) it runs along, and the parts of it used.
#[derive(Debug, Clone, PartialEq)]
pub struct PolyEdgeSegment {
    pub object: Uuid,
    /// `ON_COMPONENT_INDEX`: its type (1 Brep vertex, 2 Brep edge, 4 Brep trim, ..) and index.
    pub component: (i32, i32),
    pub edge_domain: [f64; 2],
    pub trim_domain: [f64; 2],
    pub reversed: bool,
    /// Its own domain, and the part of the edge it uses.
    pub domain: [f64; 2],
    pub curve_domain: [f64; 2],
}
