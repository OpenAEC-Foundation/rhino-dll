//! The neutral model the 3dm reader produces: the layers, materials, bitmaps and block
//! definitions of a Rhino file, and its objects with their attributes, user text and geometry,
//! as meshes, polylines, points or block instances. Exact geometry (polysurfaces, surfaces,
//! extrusions, SubD, curves, texts and hatches) is turned into these by the reader.

use glam::{DMat4, DVec3};
use std::collections::BTreeMap;
pub use threedm_core::{Uuid, Versions};

/// A layer (`ON_Layer`); its full path follows the parents by id.
#[derive(Debug, Clone)]
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
}

/// A material (`ON_Material`): its colours, shine, reflectivity, index of refraction and
/// transparency, its textures, and its physically based parameters when it has them.
#[derive(Debug, Clone, Default)]
pub struct Material {
    pub index: i32,
    pub name: String,
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

/// A viewport of the file's views (`ON_3dmView`); the first is the view that was active.
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
}

/// A Rhino file as read.
#[derive(Debug, Default)]
pub struct File<'a> {
    pub versions: Versions,
    pub application: String,
    /// Metres per model unit, and the unit's name.
    pub meters: f64,
    pub unit: &'static str,
    pub layers: Vec<Layer>,
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
    /// Records that could not be read, by table.
    pub damaged: BTreeMap<&'static str, usize>,
}
