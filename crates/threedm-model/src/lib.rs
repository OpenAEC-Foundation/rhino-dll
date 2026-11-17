//! The neutral model the 3dm reader produces: the layers, materials and block definitions of a
//! Rhino file, and its objects with their attributes, user text and geometry, as meshes,
//! polylines, points or block instances. Exact geometry (polysurfaces, surfaces, extrusions,
//! SubD, curves, texts and hatches) is turned into these by the reader.

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

/// A material (`ON_Material`): its diffuse colour and transparency.
#[derive(Debug, Clone)]
pub struct Material {
    pub index: i32,
    pub name: String,
    pub diffuse: [u8; 4],
    pub transparency: f64,
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
}

impl Default for Attributes {
    fn default() -> Self {
        Attributes { id: Uuid::default(), name: String::new(), layer: 0, material: -1, color: [0, 0, 0, 0], color_source: 0, material_source: 0, visible: true, mode: 0, space: 1, linetype: -1, linetype_source: 0 }
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
    pub blocks: Vec<BlockDefinition>,
    pub objects: Vec<Object<'a>>,
    /// The viewports of the file's views, the active one first.
    pub views: Vec<View>,
    /// The views saved by name (`NamedView`).
    pub named_views: Vec<View>,
    /// The lights of the light table (and of the object table, where old files keep them).
    pub lights: Vec<Light>,
    /// Records that could not be read, by table.
    pub damaged: BTreeMap<&'static str, usize>,
}
