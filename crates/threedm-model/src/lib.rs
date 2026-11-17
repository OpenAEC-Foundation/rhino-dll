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
    /// Not drawn: why, for the count in the project information.
    Skipped(&'static str),
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
    /// Records that could not be read, by table.
    pub damaged: BTreeMap<&'static str, usize>,
}
