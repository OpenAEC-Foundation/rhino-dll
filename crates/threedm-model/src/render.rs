//! Rendering data: the render settings of a file, its render content (materials, environments
//! and textures of the RDK document), and the decals and mesh modifiers of objects.

use crate::{File, Object, Uuid};

impl File<'_> {
    /// The render content of a material (by its index), when the material simulates one.
    pub fn render_material(&self, material: i32) -> Option<&RenderContent> {
        let m = self.materials.iter().find(|m| m.index == material)?;
        if m.render_material.is_nil() {
            return None;
        }
        self.render_content.iter().find(|c| c.id == m.render_material)
    }

    /// The material index an object is rendered with: its own, or its layer's (`-1` the
    /// default material; by parent, of objects in blocks, its own too).
    pub fn object_material(&self, o: &Object<'_>) -> i32 {
        match o.attributes.material_source {
            0 => self.layers.iter().find(|l| l.index == o.attributes.layer).map_or(-1, |l| l.material),
            _ => o.attributes.material,
        }
    }
}

/// A value of a parameter of render content, a decal or a mesh modifier, as the type its XML
/// gives it (`ON_XMLVariant`).
#[derive(Debug, Clone, PartialEq)]
pub enum RenderValue {
    Bool(bool),
    Integer(i64),
    /// `float` and `double`.
    Number(f64),
    Text(String),
    Uuid(Uuid),
    /// Red, green, blue and alpha, 0 to 1 (`color`).
    Color([f32; 4]),
    /// `vector2d`, `vector3d`, `point4d` and `matrix` (16 numbers, row by row).
    Numbers(Vec<f64>),
    /// No type and no value.
    Null,
}

/// Render content of the RDK document (`ON_RenderContent`): a render material, environment or
/// texture, with the content under it (`children`, each in its child slot).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RenderContent {
    /// "material", "environment" or "texture".
    pub kind: &'static str,
    pub type_name: String,
    pub type_id: Uuid,
    pub id: Uuid,
    pub name: String,
    pub render_engine: Uuid,
    pub plug_in: Uuid,
    pub group: Uuid,
    pub notes: String,
    pub tags: String,
    pub hidden: bool,
    pub reference: bool,
    pub auto_delete: bool,
    /// The slot of its parent it is in; empty for top-level content.
    pub child_slot: String,
    /// Nil for top-level content.
    pub parent: Uuid,
    pub children: Vec<Uuid>,
    /// Its parameters by name (the V8 section when there is one, else the older one).
    pub parameters: Vec<(String, RenderValue)>,
    /// The `ON_Material`, `ON_Texture` or `ON_Environment` Rhino simulates it with, by name.
    pub simulation: Vec<(String, RenderValue)>,
}

impl RenderContent {
    /// A parameter by name.
    pub fn parameter(&self, name: &str) -> Option<&RenderValue> {
        self.parameters.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }
}

/// The rendering settings of a file: `ON_3dmRenderSettings` and the parts of the RDK
/// document (environments, ground plane, sun, skylight, linear workflow, dithering, safe frame,
/// post effects, render channels).
#[derive(Debug, Clone, PartialEq)]
pub struct RenderSettings {
    /// The image size is its own (not the viewport's): width, height (pixels), dots per inch
    /// and the unit (`ON::LengthUnitSystem`) of its print size.
    pub custom_image_size: bool,
    pub image_width: i32,
    pub image_height: i32,
    pub image_dpi: f64,
    pub image_unit: i32,
    /// Colours as stored: red, green, blue and alpha (0 opaque).
    pub ambient: [u8; 4],
    /// 0 solid colour, 1 wallpaper (`wallpaper`), 2 gradient (top `background_color`, bottom
    /// `background_bottom_color`), 3 environment.
    pub background_style: i32,
    pub background_color: [u8; 4],
    pub background_bottom_color: [u8; 4],
    pub wallpaper: String,
    pub use_hidden_lights: bool,
    pub depth_cue: bool,
    pub flat_shade: bool,
    pub render_backfaces: bool,
    pub render_points: bool,
    pub render_curves: bool,
    pub render_isoparams: bool,
    pub render_mesh_edges: bool,
    pub render_annotations: bool,
    pub scale_background_to_fit: bool,
    pub transparent_background: bool,
    /// Antialias and shadow map styles (0 none, 1 normal, 2 best) and the shadow map's size
    /// and offset.
    pub antialias: i32,
    pub shadowmap: i32,
    pub shadowmap_width: i32,
    pub shadowmap_height: i32,
    pub shadowmap_offset: f64,
    /// What is rendered: 0 the active viewport, 1 a specific viewport, 2 a named view, 3 a
    /// snapshot; and their names.
    pub rendering_source: i32,
    pub specific_viewport: String,
    pub named_view: String,
    pub snapshot: String,
    pub force_viewport_aspect_ratio: bool,
    /// The environment of the background (as set; nil for the default), and the one of
    /// reflections when it overrides it.
    pub background_environment: Uuid,
    pub reflection_environment_on: bool,
    pub reflection_environment: Uuid,
    pub ground_plane: GroundPlane,
    pub sun: Sun,
    pub skylight: Skylight,
    pub linear_workflow: LinearWorkflow,
    pub dithering: Dithering,
    pub safe_frame: SafeFrame,
    pub post_effects: Vec<PostEffect>,
    /// Render channels: automatic (`false`) or the custom list.
    pub render_channels_custom: bool,
    pub render_channels: Vec<Uuid>,
}

impl Default for RenderSettings {
    /// `ON_3dmRenderSettings::Default`, and the defaults of the RDK document's parts.
    fn default() -> Self {
        RenderSettings {
            custom_image_size: false,
            image_width: 800,
            image_height: 600,
            image_dpi: 72.0,
            image_unit: 8,
            ambient: [0, 0, 0, 0],
            background_style: 0,
            background_color: [255, 255, 255, 0],
            background_bottom_color: [160, 160, 160, 0],
            wallpaper: String::new(),
            use_hidden_lights: false,
            depth_cue: false,
            flat_shade: false,
            render_backfaces: true,
            render_points: false,
            render_curves: false,
            render_isoparams: false,
            render_mesh_edges: false,
            render_annotations: false,
            scale_background_to_fit: false,
            transparent_background: false,
            antialias: 1,
            shadowmap: 1,
            shadowmap_width: 1000,
            shadowmap_height: 1000,
            shadowmap_offset: 0.75,
            rendering_source: 0,
            specific_viewport: String::new(),
            named_view: String::new(),
            snapshot: String::new(),
            force_viewport_aspect_ratio: false,
            background_environment: Uuid::default(),
            reflection_environment_on: false,
            reflection_environment: Uuid::default(),
            ground_plane: GroundPlane::default(),
            sun: Sun::default(),
            skylight: Skylight::default(),
            linear_workflow: LinearWorkflow::default(),
            dithering: Dithering::default(),
            safe_frame: SafeFrame::default(),
            post_effects: Vec::new(),
            render_channels_custom: false,
            render_channels: Vec::new(),
        }
    }
}

/// The ground plane (`ON_GroundPlane`): an infinite plane at an altitude, with a material.
#[derive(Debug, Clone, PartialEq)]
pub struct GroundPlane {
    pub on: bool,
    pub altitude: f64,
    /// Its altitude follows the lowest point of the model.
    pub auto_altitude: bool,
    /// It shows only shadows, not its material.
    pub shadow_only: bool,
    pub show_underside: bool,
    /// The render material (nil for none).
    pub material: Uuid,
    pub texture_offset: [f64; 2],
    pub texture_size: [f64; 2],
    /// Degrees.
    pub texture_rotation: f64,
    pub texture_offset_locked: bool,
    pub texture_size_locked: bool,
}

impl Default for GroundPlane {
    fn default() -> Self {
        GroundPlane {
            on: false,
            altitude: 0.0,
            auto_altitude: false,
            shadow_only: true,
            show_underside: false,
            material: Uuid::default(),
            texture_offset: [0.0; 2],
            texture_size: [0.0; 2],
            texture_rotation: 0.0,
            texture_offset_locked: false,
            texture_size_locked: false,
        }
    }
}

/// The sun (`ON_Sun`): placed by hand (azimuth and altitude) or by a local date, time and
/// place. `azimuth` and `altitude` are its position either way (computed as openNURBS does for
/// the latter), `color` the colour of its light at that altitude and `direction` where its
/// light goes.
#[derive(Debug, Clone, PartialEq)]
pub struct Sun {
    pub on: bool,
    pub manual: bool,
    /// Degrees: compass azimuth (clockwise from north) and altitude above the horizon.
    pub azimuth: f64,
    pub altitude: f64,
    /// Degrees from the world x axis to north (90: north is +y).
    pub north: f64,
    pub latitude: f64,
    pub longitude: f64,
    /// Hours from UTC.
    pub time_zone: f64,
    pub daylight_saving_on: bool,
    pub daylight_saving_minutes: i32,
    /// The local date and time of day (hours).
    pub year: i32,
    pub month: i32,
    pub day: i32,
    pub hours: f64,
    pub intensity: f64,
    pub shadow_intensity: f64,
    /// Red, green, blue and alpha, 0 to 1.
    pub color: [f32; 4],
    /// World, unit length.
    pub direction: [f64; 3],
}

impl Default for Sun {
    fn default() -> Self {
        Sun {
            on: false,
            manual: false,
            azimuth: 0.0,
            altitude: 0.0,
            north: 90.0,
            latitude: 0.0,
            longitude: 0.0,
            time_zone: 0.0,
            daylight_saving_on: false,
            daylight_saving_minutes: 0,
            year: 2000,
            month: 1,
            day: 1,
            hours: 12.0,
            intensity: 1.0,
            shadow_intensity: 1.0,
            color: [1.0; 4],
            direction: [0.0, 0.0, -1.0],
        }
    }
}

/// Skylight (`ON_Skylight`), with its own environment when it overrides the background's.
#[derive(Debug, Clone, PartialEq)]
pub struct Skylight {
    pub on: bool,
    pub shadow_intensity: f64,
    pub environment_on: bool,
    pub environment: Uuid,
}

impl Default for Skylight {
    fn default() -> Self {
        Skylight { on: false, shadow_intensity: 1.0, environment_on: false, environment: Uuid::default() }
    }
}

/// Linear workflow (`ON_LinearWorkflow`): colours and textures linearised before rendering
/// (`pre_process`), gamma applied after (`post_process`).
#[derive(Debug, Clone, PartialEq)]
pub struct LinearWorkflow {
    pub pre_process: bool,
    pub post_process: bool,
    pub gamma: f64,
}

impl Default for LinearWorkflow {
    fn default() -> Self {
        LinearWorkflow { pre_process: false, post_process: true, gamma: 2.2 }
    }
}

/// Dithering (`ON_Dithering`) and its method, "floyd-steinberg" or "simple-noise".
#[derive(Debug, Clone, PartialEq)]
pub struct Dithering {
    pub on: bool,
    pub method: &'static str,
}

impl Default for Dithering {
    fn default() -> Self {
        Dithering { on: false, method: "simple-noise" }
    }
}

/// The safe frame (`ON_SafeFrame`): its live, action and title frames.
#[derive(Debug, Clone, PartialEq)]
pub struct SafeFrame {
    pub on: bool,
    pub perspective_only: bool,
    /// The 4:3 field grid.
    pub field_grid_on: bool,
    pub live_frame_on: bool,
    pub action_frame: SafeFrameFrame,
    pub title_frame: SafeFrameFrame,
}

/// A frame of the safe frame: on, its scales and whether they are linked.
#[derive(Debug, Clone, PartialEq)]
pub struct SafeFrameFrame {
    pub on: bool,
    pub linked: bool,
    pub x_scale: f64,
    pub y_scale: f64,
}

impl Default for SafeFrame {
    fn default() -> Self {
        SafeFrame {
            on: false,
            perspective_only: false,
            field_grid_on: false,
            live_frame_on: false,
            action_frame: SafeFrameFrame { on: false, linked: false, x_scale: 0.9, y_scale: 0.9 },
            title_frame: SafeFrameFrame { on: false, linked: false, x_scale: 0.8, y_scale: 0.8 },
        }
    }
}

/// A post effect (`ON_PostEffect`): its id, name, stage ("early", "tone-mapping", "late"),
/// whether it is on and shown, and its parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct PostEffect {
    pub id: Uuid,
    pub name: String,
    pub stage: &'static str,
    pub on: bool,
    pub shown: bool,
    pub parameters: Vec<(String, RenderValue)>,
}

/// A decal (`ON_Decal`): a texture projected onto an object.
#[derive(Debug, Clone, PartialEq)]
pub struct Decal {
    /// The render texture.
    pub texture: Uuid,
    /// "planar", "cylindrical", "spherical", "uv" or "none".
    pub mapping: String,
    /// "forward", "backward", "both" or "none".
    pub projection: String,
    pub map_to_inside: bool,
    /// 0 opaque to 1.
    pub transparency: f64,
    pub visible: bool,
    pub origin: [f64; 3],
    /// Its axes; their lengths are its size.
    pub vector_up: [f64; 3],
    pub vector_across: [f64; 3],
    /// Height and radius of a cylindrical or spherical decal, and the sweeps (start, end) it
    /// covers horizontally and vertically.
    pub height: f64,
    pub radius: f64,
    pub horizontal_sweep: [f64; 2],
    pub vertical_sweep: [f64; 2],
    /// Minimum u and v, maximum u and v of a UV decal.
    pub uv_bounds: [f64; 4],
}

/// The mesh modifiers of an object (`ON_MeshModifiers`): what Rhino changes its render mesh
/// with. Only read: the meshes are not changed.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MeshModifiers {
    pub displacement: Option<Displacement>,
    pub edge_softening: Option<EdgeSoftening>,
    pub thickening: Option<Thickening>,
    pub curve_piping: Option<CurvePiping>,
    pub shut_lining: Option<ShutLining>,
}

impl MeshModifiers {
    pub fn is_empty(&self) -> bool {
        *self == MeshModifiers::default()
    }
}

/// Displacement (`ON_Displacement`) by a texture.
#[derive(Debug, Clone, PartialEq)]
pub struct Displacement {
    pub on: bool,
    pub texture: Uuid,
    pub mapping_channel: i32,
    pub black_point: f64,
    pub white_point: f64,
    /// The initial quality (`sweep-pitch`).
    pub sweep_pitch: i32,
    pub refine_steps: i32,
    pub refine_sensitivity: f64,
    pub face_limit_on: bool,
    pub face_limit: i32,
    pub fairing_on: bool,
    pub fairing: i32,
    pub post_weld_angle: f64,
    /// Megabytes.
    pub memory_limit: i32,
    /// 0 the default formula, 1 by the absolute tolerance.
    pub sweep_formula: i32,
}

/// Edge softening (`ON_EdgeSoftening`).
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeSoftening {
    pub on: bool,
    /// The softening radius.
    pub softening: f64,
    pub chamfer: bool,
    pub faceted: bool,
    /// Degrees.
    pub edge_angle_threshold: f64,
    pub force_softening: bool,
}

/// Thickening (`ON_Thickening`).
#[derive(Debug, Clone, PartialEq)]
pub struct Thickening {
    pub on: bool,
    pub distance: f64,
    pub solid: bool,
    pub offset_only: bool,
    pub both_sides: bool,
}

/// Curve piping (`ON_CurvePiping`).
#[derive(Debug, Clone, PartialEq)]
pub struct CurvePiping {
    pub on: bool,
    pub radius: f64,
    pub segments: i32,
    pub faceted: bool,
    pub accuracy: i32,
    /// "none", "flat", "box" or "dome".
    pub cap_type: &'static str,
}

/// Shut lining (`ON_ShutLining`) along curves.
#[derive(Debug, Clone, PartialEq)]
pub struct ShutLining {
    pub on: bool,
    pub faceted: bool,
    pub auto_update: bool,
    pub force_update: bool,
    pub curves: Vec<ShutLiningCurve>,
}

/// A curve of shut lining: the curve's id and its profile.
#[derive(Debug, Clone, PartialEq)]
pub struct ShutLiningCurve {
    pub id: Uuid,
    pub enabled: bool,
    pub radius: f64,
    pub profile: i32,
    pub pull: bool,
    pub is_bump: bool,
}
