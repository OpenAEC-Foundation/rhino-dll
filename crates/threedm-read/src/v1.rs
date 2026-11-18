//! Rhino 1 files (archive version 1, 1998): no object table, but top-level chunks, as openNURBS
//! reads them (`Read3dmV1Layer`, `Read3dmV1Object`, `Read3dmV1AttributesOrMaterial`): layers
//! (name, colour, hidden), the unit system, and objects, each its geometry followed by its
//! attributes (layer by index, display colour, name). Meshes (compressed: points as fractions of
//! their bounding box), points (with their arrows and dots), curves (NURBS segments, drawn as one
//! polyline) and faces and shells (trimmed NURBS surfaces, meshed as Breps are) are read.

use super::tessellate::{legacy_brep, nurbs_curve, nurbs_surface, polyline, Curve, LegacyFace, LegacyTrim};
use super::{err, Cur, Mesh, R, RhinoLayer, RhinoObject, MAX_VERTICES, TOO_MANY_VERTICES};
use glam::DVec3;
use threedm_model::{Attributes, File, Geometry};

const TCODE_ENDOFTABLE: u32 = 0xFFFF_FFFF;
const TCODE_LAYER: u32 = 0x0040_0010;
const TCODE_LAYERNAME: u32 = 0x0040_0011;
const TCODE_RGB: u32 = 0x8040_0001;
const TCODE_LAYERSTATE: u32 = 0x8040_0033;
const TCODE_UNIT_AND_TOLERANCES: u32 = 0x0200_0010;
const TCODE_RH_POINT: u32 = 0x0010_0001;
const TCODE_MESH_OBJECT: u32 = 0x0010_0015;
const TCODE_COMPRESSED_MESH_GEOMETRY: u32 = 0x0010_0017;
const TCODE_LEGACY_SHL: u32 = 0x0001_0003;
const TCODE_LEGACY_FAC: u32 = 0x0001_0004;
const TCODE_LEGACY_CRV: u32 = 0x0001_0008;
const TCODE_LEGACY_SPL: u32 = 0x0001_0009;
const TCODE_LEGACY_CRVSTUFF: u32 = 0x0001_0108;
const TCODE_LEGACY_SPLSTUFF: u32 = 0x0001_0109;
const TCODE_LEGACY_BND: u32 = 0x0001_0005;
const TCODE_LEGACY_TRM: u32 = 0x0001_0006;
const TCODE_LEGACY_SRF: u32 = 0x0001_0007;
const TCODE_LEGACY_SHLSTUFF: u32 = 0x0001_0103;
const TCODE_LEGACY_FACSTUFF: u32 = 0x0001_0104;
const TCODE_LEGACY_BNDSTUFF: u32 = 0x0001_0105;
const TCODE_LEGACY_TRMSTUFF: u32 = 0x0001_0106;
const TCODE_LEGACY_SRFSTUFF: u32 = 0x0001_0107;
const TCODE_LAYERREF: u32 = 0xA000_0001;
const TCODE_RGBDISPLAY: u32 = 0x8040_0006;
const TCODE_NAME: u32 = 0x0200_0001;

/// Reads the chunks of a Rhino 1 file from `top` (just past its header) into `file`.
/// `vertices` counts the mesh vertices read, at most [`MAX_VERTICES`].
pub(super) fn read<'a>(mut top: Cur<'a>, file: &mut File<'a>, vertices: &mut usize) -> R<()> {
    while top.left() > 0 {
        let Ok(chunk) = top.chunk() else {
            *file.damaged.entry("end of file").or_default() += 1;
            break;
        };
        let mut body = chunk.body;
        match chunk.typecode {
            TCODE_LAYER => {
                let index = file.layers.len() as i32;
                match read_layer(body, index) {
                    Ok(layer) => file.layers.push(layer),
                    Err(_) => *file.damaged.entry("layers").or_default() += 1,
                }
            }
            TCODE_UNIT_AND_TOLERANCES => {
                if let Ok(Some((meters, unit))) = read_units(body) {
                    file.meters = meters;
                    file.unit = unit;
                }
            }
            TCODE_MESH_OBJECT => {
                let geometry = match read_mesh(&mut body, vertices) {
                    Ok(mesh) => Geometry::Meshes(vec![mesh], "Mesh"),
                    Err(e) if e == TOO_MANY_VERTICES => Geometry::Skipped("meshes beyond the vertex limit"),
                    Err(_) => Geometry::Skipped("Rhino 1 meshes that could not be read"),
                };
                let (attributes, _) = read_attributes(&mut body, None);
                file.objects.push(RhinoObject { geometry, attributes, user: Vec::new(), stroke: None, font: None, unmodified: None });
            }
            TCODE_RH_POINT => {
                // a point, or by its extra data an arrow (a curve, as Rhino 9 converts it) or
                // a text dot
                let point = body.take(24);
                let extra = xdata(body.clone());
                let (attributes, _) = read_attributes(&mut body, None);
                let mut user = Vec::new();
                let geometry = match (point, extra) {
                    (Ok(p), Some(Extra::Arrow(v))) => {
                        let at = DVec3::from_array(std::array::from_fn(|k| f64::from_le_bytes(p[8 * k..8 * k + 8].try_into().unwrap())));
                        Geometry::Lines(vec![vec![at, at + v]], "Curve")
                    }
                    (Ok(points), Some(Extra::Dot(text))) => {
                        user.push(("Text".to_string(), text));
                        Geometry::Points { points, colors: None, kind: "Text dot" }
                    }
                    (Ok(points), None) => Geometry::Points { points, colors: None, kind: "Point" },
                    (Err(_), _) => Geometry::Skipped("Rhino 1 points that could not be read"),
                };
                file.objects.push(RhinoObject { geometry, attributes, user, stroke: None, font: None, unmodified: None });
            }
            TCODE_LEGACY_CRV => {
                // its attributes, then the curve
                let (attributes, stuff) = read_attributes(&mut body, Some(TCODE_LEGACY_CRVSTUFF));
                let geometry = match stuff.map(read_curve) {
                    Some(Ok(points)) => Geometry::Lines(vec![points], "Curve"),
                    _ => Geometry::Skipped("Rhino 1 curves that could not be read"),
                };
                file.objects.push(RhinoObject { geometry, attributes, user: Vec::new(), stroke: None, font: None, unmodified: None });
            }
            TCODE_LEGACY_FAC | TCODE_LEGACY_SHL => {
                // its attributes, then the face or the faces of the shell, meshed
                let shell = chunk.typecode == TCODE_LEGACY_SHL;
                let stuff = if shell { TCODE_LEGACY_SHLSTUFF } else { TCODE_LEGACY_FACSTUFF };
                let (attributes, body) = read_attributes(&mut body, Some(stuff));
                let faces = body.ok_or(()).and_then(|b| if shell { read_shell(b) } else { read_face(b).map(|(f, size)| (vec![f], size)) }.map_err(|_| ()));
                let geometry = match faces {
                    Ok((faces, size)) => match legacy_brep(faces, size, vertices) {
                        Ok(meshes) if !meshes.is_empty() => {
                            let kind = if shell { "Polysurface" } else { "Surface" };
                            Geometry::Meshes(meshes, kind)
                        }
                        Err(e) if e == TOO_MANY_VERTICES => Geometry::Skipped("meshes beyond the vertex limit"),
                        _ => Geometry::Skipped("Rhino 1 surfaces that could not be meshed"),
                    },
                    Err(_) => Geometry::Skipped("Rhino 1 surfaces that could not be read"),
                };
                file.objects.push(RhinoObject { geometry, attributes, user: Vec::new(), stroke: None, font: None, unmodified: None });
            }
            _ => {}
        }
    }
    // Rhino 1 objects have no ids: one of their own from their place in the file, as Rhino
    // gives them new ones on reading
    for (k, o) in file.objects.iter_mut().enumerate() {
        let mut id = [0u8; 16];
        id[..8].copy_from_slice(&(k as u64 + 1).to_le_bytes());
        id[8..].copy_from_slice(b"rhino1ob");
        o.attributes.id = super::Uuid(id);
    }
    Ok(())
}


/// `Read3dmV1Layer`: its name, colour and state (1 hidden) in chunks.
fn read_layer(mut c: Cur<'_>, index: i32) -> R<RhinoLayer> {
    let mut layer = RhinoLayer { index, name: format!("Layer {index}"), id: Default::default(), parent: Default::default(), color: [0, 0, 0, 255], visible: true, material: -1, linetype: -1, section_style: -1, ..RhinoLayer::default() };
    while c.left() > 0 {
        let chunk = c.chunk()?;
        match chunk.typecode {
            TCODE_ENDOFTABLE => break,
            TCODE_LAYERNAME => {
                let mut b = chunk.body;
                let n = b.count(1)?;
                let name: String = b.take(n)?.iter().take_while(|&&c| c != 0).map(|&c| c as char).collect();
                if !name.trim().is_empty() {
                    layer.name = name;
                }
            }
            TCODE_RGB => layer.color = rgb(chunk.value),
            TCODE_LAYERSTATE => layer.visible = chunk.value != 1,
            _ => {}
        }
    }
    Ok(layer)
}

/// A colour as Rhino 1 stores it: red in the low byte.
fn rgb(v: i64) -> [u8; 4] {
    let u = v as u32;
    [(u & 0xFF) as u8, ((u >> 8) & 0xFF) as u8, ((u >> 16) & 0xFF) as u8, 255]
}

/// `ON_3dmSettings_Read_v1_TCODE_UNIT_AND_TOLERANCES`: a version and Rhino 1's unit system.
fn read_units(mut c: Cur<'_>) -> R<Option<(f64, &'static str)>> {
    c.i32()?;
    Ok(match c.i32()? {
        1 => Some((1e-6, "microns")),
        2 => Some((0.001, "millimetres")),
        3 => Some((0.01, "centimetres")),
        4 => Some((1.0, "metres")),
        5 => Some((1000.0, "kilometres")),
        6 => Some((2.54e-8, "microinches")),
        7 => Some((2.54e-5, "mils")),
        8 => Some((0.0254, "inches")),
        9 => Some((0.3048, "feet")),
        10 => Some((1609.344, "miles")),
        _ => None,
    })
}

/// `ReadV1_TCODE_MESH_OBJECT`: a compressed mesh: point and face counts, whether it has normals
/// and texture coordinates, its bounding box, the points as fractions of it (three u16 each),
/// the faces (four u16, or i32 from 65535 points on), then normals (three i8) and texture
/// coordinates.
fn read_mesh(c: &mut Cur<'_>, vertices: &mut usize) -> R<Mesh> {
    let chunk = c.chunk()?;
    if chunk.typecode != TCODE_COMPRESSED_MESH_GEOMETRY {
        return err("a Rhino 1 mesh of an unknown kind");
    }
    let mut b = chunk.body;
    let points = b.i32()?;
    let faces = b.i32()?;
    if points <= 0 || faces <= 0 {
        return err("an empty Rhino 1 mesh");
    }
    let (points, faces) = (points as usize, faces as usize);
    if points > MAX_VERTICES - *vertices {
        return Err(TOO_MANY_VERTICES.into());
    }
    let has_normals = b.i32()? != 0;
    b.i32()?; // texture coordinates
    let low = [b.f64()?, b.f64()?, b.f64()?];
    let high = [b.f64()?, b.f64()?, b.f64()?];
    if points.saturating_mul(6) > b.left() {
        return err("a Rhino 1 mesh larger than its record");
    }
    let step: [f64; 3] = std::array::from_fn(|k| (high[k] - low[k]) / 65535.0);
    let mut positions = Vec::with_capacity(points);
    for _ in 0..points {
        let q = b.take(6)?;
        positions.push(std::array::from_fn(|k| (step[k] * u16::from_le_bytes([q[2 * k], q[2 * k + 1]]) as f64 + low[k]) as f32));
    }
    let wide = points >= 65535;
    if faces.saturating_mul(if wide { 16 } else { 8 }) > b.left() {
        return err("a Rhino 1 mesh larger than its record");
    }
    let mut list = Vec::with_capacity(faces);
    let mut triangles = 0;
    for _ in 0..faces {
        let f: [u32; 4] = if wide {
            [b.u32()?, b.u32()?, b.u32()?, b.u32()?]
        } else {
            let q = b.take(8)?;
            std::array::from_fn(|k| u16::from_le_bytes([q[2 * k], q[2 * k + 1]]) as u32)
        };
        if f.iter().any(|&i| i as usize >= points) {
            continue;
        }
        triangles += if f[2] == f[3] { 1 } else { 2 };
        list.push(f);
    }
    let normals = if has_normals && b.left() >= 3 * points {
        let n = b.take(3 * points)?;
        Some(n.chunks_exact(3).map(|v| [v[0] as i8 as f32 / 127.0, v[1] as i8 as f32 / 127.0, v[2] as i8 as f32 / 127.0]).collect())
    } else {
        None
    };
    *vertices += points;
    Ok(Mesh { positions, normals, colors: None, texture_coordinates: None, faces: list, triangles, color: None, computed: false })
}

/// `Read3dmV1AttributesOrMaterial`: the chunks of an object's attributes: its layer (by
/// index), display colour and name; and the body of the chunk `stuff` (its geometry) when it
/// meets it.
fn read_attributes<'a>(c: &mut Cur<'a>, stuff: Option<u32>) -> (Attributes, Option<Cur<'a>>) {
    let mut a = Attributes::default();
    let mut found = None;
    while c.left() > 0 {
        let Ok(chunk) = c.chunk() else { break };
        match chunk.typecode {
            TCODE_ENDOFTABLE => break,
            t if Some(t) == stuff && found.is_none() => found = Some(chunk.body),
            TCODE_LAYERREF => a.layer = chunk.value as i32,
            TCODE_RGBDISPLAY => {
                a.color = rgb(chunk.value);
                a.color_source = 1;
            }
            TCODE_NAME => {
                let mut b = chunk.body;
                if let Ok(n) = b.count(1) {
                    if let Ok(s) = b.take(n) {
                        a.name = s.iter().take_while(|&&c| c != 0).map(|&c| c as char).collect();
                    }
                }
            }
            _ => {}
        }
    }
    (a, found)
}

const TCODE_XDATA: u32 = 0x4000_0001;

/// What Rhino 1 adds to a point in its extra data.
enum Extra {
    /// An arrow from the point along this vector.
    Arrow(DVec3),
    /// A text dot with this text.
    Dot(String),
}

/// The extra data (`TCODE_XDATA`: `RhAnnotateArrow`, `RhAnnotateDot`) among an object's
/// attribute chunks.
fn xdata(mut c: Cur<'_>) -> Option<Extra> {
    while c.left() > 0 {
        let chunk = c.chunk().ok()?;
        if chunk.typecode != TCODE_XDATA {
            continue;
        }
        let mut b = chunk.body;
        let (id_size, data_size) = (b.count(1).ok()?, b.count(1).ok()?);
        let id: String = b.take(id_size).ok()?.iter().take_while(|&&c| c != 0).map(|&c| c as char).collect();
        let data = b.take(data_size).ok()?;
        if id.eq_ignore_ascii_case("RhAnnotateArrow") && data.len() == 24 {
            let v = DVec3::from_array(std::array::from_fn(|k| f64::from_le_bytes(data[8 * k..8 * k + 8].try_into().unwrap())));
            if v.length() > 1e-12 && v.is_finite() {
                return Some(Extra::Arrow(v));
            }
        } else if id.eq_ignore_ascii_case("RhAnnotateDot") {
            return Some(Extra::Dot(data.iter().take_while(|&&c| c != 0).map(|&c| c as char).collect()));
        }
    }
    None
}

/// The body of the first chunk `tcode` among the chunks at `c` (`BeginRead3dmLEGACYSTUFF`).
fn legacy_stuff<'a>(mut c: Cur<'a>, tcode: u32) -> R<Cur<'a>> {
    while c.left() > 0 {
        let chunk = c.chunk()?;
        if chunk.typecode == tcode {
            return Ok(chunk.body);
        }
    }
    err("a Rhino 1 object without its geometry")
}

/// At most this many faces, loops of a face and trims of a loop are read.
const MAX_PARTS: usize = 100_000;

/// `ReadV1_LegacyShellStuff`: outer, face count, bounding box, glue of the faces' mated trims,
/// then the faces. With the size across its bounding box.
fn read_shell(mut c: Cur<'_>) -> R<(Vec<LegacyFace>, f64)> {
    c.i32()?; // outer
    let count = c.count(1)?;
    let low = point(&mut c)?;
    let high = point(&mut c)?;
    let twins = c.count(2)?;
    c.take(2 * twins)?; // which mated trims meet
    if count > MAX_PARTS {
        return err("a Rhino 1 shell of too many faces");
    }
    let mut faces = Vec::with_capacity(count);
    for _ in 0..count {
        let chunk = c.chunk()?;
        if chunk.typecode != TCODE_LEGACY_FAC {
            return err("a Rhino 1 shell without its faces");
        }
        let face = legacy_stuff(chunk.body, TCODE_LEGACY_FACSTUFF)?;
        faces.push(read_face(face)?.0);
    }
    Ok((faces, (high - low).length()))
}

fn point(c: &mut Cur<'_>) -> R<DVec3> {
    Ok(DVec3::new(c.f64()?, c.f64()?, c.f64()?))
}

/// `ReadV1_LegacyFaceStuff`: reversed, type, loop count (twice, plus one with an outer loop),
/// bounding box, glue of its seams, the surface, then the loops. With the size across its
/// bounding box.
fn read_face(mut c: Cur<'_>) -> R<(LegacyFace, f64)> {
    let reversed = c.i32()? == 1;
    c.i32()?; // type
    let loops = (c.i32()?.max(0) / 2) as usize;
    let low = point(&mut c)?;
    let high = point(&mut c)?;
    let twins = c.count(2)?;
    c.take(2 * twins)?; // which seams meet
    if loops > MAX_PARTS {
        return err("a Rhino 1 face of too many loops");
    }
    let srf = c.chunk()?;
    if srf.typecode != TCODE_LEGACY_SRF {
        return err("a Rhino 1 face without its surface");
    }
    let surface = read_surface(legacy_stuff(srf.body, TCODE_LEGACY_SRFSTUFF)?)?;
    let mut face = LegacyFace { surface, reversed, loops: Vec::with_capacity(loops) };
    for _ in 0..loops {
        let bnd = c.chunk()?;
        if bnd.typecode != TCODE_LEGACY_BND {
            return err("a Rhino 1 face without its loops");
        }
        face.loops.push(read_loop(legacy_stuff(bnd.body, TCODE_LEGACY_BNDSTUFF)?)?);
    }
    Ok((face, (high - low).length()))
}

/// `ReadV1_LegacyLoopStuff`: its trim count, type (-1 slit, 0 outer, 1 inner), its box in the
/// surface's parameters, then the trims.
fn read_loop(mut c: Cur<'_>) -> R<(i32, Vec<LegacyTrim>)> {
    let count = c.count(1)?;
    let kind = match c.i32()? {
        0 => 1,
        1 => 2,
        -1 => 3,
        _ => return err("a Rhino 1 loop of an unknown kind"),
    };
    c.take(4 * 8)?; // parameter box
    if count > MAX_PARTS {
        return err("a Rhino 1 loop of too many trims");
    }
    let mut trims = Vec::with_capacity(count);
    for _ in 0..count {
        let trm = c.chunk()?;
        if trm.typecode != TCODE_LEGACY_TRM {
            return err("a Rhino 1 loop without its trims");
        }
        trims.push(read_trim(legacy_stuff(trm.body, TCODE_LEGACY_TRMSTUFF)?)?);
    }
    Ok((kind, trims))
}

/// `ReadV1_LegacyTrimStuff`: flags (bit 0: it has an edge), against its edge, continuity,
/// monotonicity, tolerances, its 2d curve and, with an edge, the edge's 3d curve.
fn read_trim(mut c: Cur<'_>) -> R<LegacyTrim> {
    let flags = c.u8()?;
    let against = c.i32()? != 0;
    c.i32()?; // continuity
    c.i32()?; // monotonicity
    c.take(2 * 8)?; // tolerances
    let curve = read_curve_chunk(&mut c)?;
    let edge = if flags % 2 == 1 { Some(read_curve_chunk(&mut c)?) } else { None };
    Ok(LegacyTrim { curve, edge, against })
}

/// A `TCODE_LEGACY_CRV` chunk at `c` as a curve (its segments one poly curve).
fn read_curve_chunk(c: &mut Cur<'_>) -> R<Curve> {
    let crv = c.chunk()?;
    if crv.typecode != TCODE_LEGACY_CRV {
        return err("a Rhino 1 trim without its curve");
    }
    let segments = read_segments(legacy_stuff(crv.body, TCODE_LEGACY_CRVSTUFF)?)?;
    super::tessellate::poly_curve(segments)
}

/// `ReadV1_TCODE_LEGACY_SRFSTUFF`: a NURBS surface: dimension, form, degrees, span counts,
/// rational, closed, singular, bounding box, knots and control points (a row of v per u).
fn read_surface(mut c: Cur<'_>) -> R<super::tessellate::Surface> {
    let dim = c.u8()? as usize;
    if dim != 2 && dim != 3 {
        return err("a Rhino 1 surface of an unknown dimension");
    }
    c.u8()?; // form
    let order = [c.u8()? as usize + 1, c.u8()? as usize + 1];
    let spans = [u16::from_le_bytes(c.array()?) as usize, u16::from_le_bytes(c.array()?) as usize];
    if order.iter().any(|&o| o < 2) || spans.iter().any(|&s| s < 1) {
        return err("a Rhino 1 surface of an unknown form");
    }
    let count = [order[0] - 1 + spans[0], order[1] - 1 + spans[1]];
    let mut rational = 0;
    for _ in 0..2 {
        match c.u8()? {
            1 => rational = 1,
            2 => rational = 2,
            _ => {}
        }
    }
    c.take(4)?; // closed and singular, both ways
    c.take(2 * dim * 8)?; // bounding box
    let k0 = (0..order[0] + count[0] - 2).map(|_| c.f64()).collect::<R<Vec<_>>>()?;
    let k1 = (0..order[1] + count[1] - 2).map(|_| c.f64()).collect::<R<Vec<_>>>()?;
    let total = count[0].checked_mul(count[1]).filter(|&n| n <= 10_000_000).ok_or("a Rhino 1 surface too large")?;
    let cv_dim = if rational > 0 { dim + 1 } else { dim };
    if total.saturating_mul(8 * cv_dim) > c.left() {
        return err("a Rhino 1 surface larger than its record");
    }
    let mut cvs = Vec::with_capacity(total);
    for _ in 0..total {
        let mut v = [0.0, 0.0, 0.0, 1.0];
        for k in 0..cv_dim {
            v[if k == dim { 3 } else { k }] = c.f64()?;
        }
        if rational == 1 {
            // euclidean control points: times the weight
            let w = v[3];
            for x in &mut v[..dim] {
                *x *= w;
            }
        }
        cvs.push(v);
    }
    nurbs_surface(order, count, [&k0, &k1], cvs)
}

/// At most this many segments of a Rhino 1 curve are read.
const MAX_SEGMENTS: usize = 10_000;

/// `ReadV1_TCODE_LEGACY_CRVSTUFF`: a curve of NURBS segments, as one polyline within Rhino's
/// display tolerance.
fn read_curve(c: Cur<'_>) -> R<Vec<DVec3>> {
    let mut points: Vec<DVec3> = Vec::new();
    for segment in read_segments(c)? {
        let part = polyline(&segment);
        // segments meet: the first point of one is the last of the one before
        let skip = usize::from(points.last().zip(part.first()).is_some_and(|(a, b)| a.distance(*b) < 1e-9));
        points.extend(part.into_iter().skip(skip));
    }
    if points.len() < 2 {
        return err("a Rhino 1 curve of no length");
    }
    Ok(points)
}

/// The NURBS segments of a `TCODE_LEGACY_CRVSTUFF`.
fn read_segments(mut c: Cur<'_>) -> R<Vec<Curve>> {
    let dim = c.u8()? as usize;
    if dim != 2 && dim != 3 {
        return err("a Rhino 1 curve of an unknown dimension");
    }
    c.u8()?; // flag
    let count = i16::from_le_bytes(c.array()?);
    if count < 1 || count as usize > MAX_SEGMENTS {
        return err("a Rhino 1 curve without segments");
    }
    c.take(2 * dim * 8)?; // bounding box
    let mut segments = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let chunk = c.chunk()?;
        if chunk.typecode != TCODE_LEGACY_SPL {
            return err("a Rhino 1 curve segment of an unknown kind");
        }
        segments.push(read_spline(legacy_stuff(chunk.body, TCODE_LEGACY_SPLSTUFF)?)?);
    }
    Ok(segments)
}

/// `ReadV1_TCODE_LEGACY_SPLSTUFF`: a NURBS segment: dimension, rational (1 euclidean, 2
/// homogeneous), order, control point count, closed, form, bounding box, knots (with a flag for
/// clamped ends from order 3 on), control points.
fn read_spline(mut c: Cur<'_>) -> R<Curve> {
    let dim = c.u8()? as usize;
    let rational = c.u8()?;
    let order = c.u8()? as usize;
    let count = u16::from_le_bytes(c.array()?) as usize;
    if !(dim == 2 || dim == 3) || rational > 2 || order < 2 || count < order {
        return err("a Rhino 1 spline of an unknown form");
    }
    c.u8()?; // closed
    c.u8()?; // form
    c.take(2 * dim * 8)?; // bounding box
    let knot_count = order + count - 2;
    let mut knots = Vec::with_capacity(knot_count);
    let clamped = if order > 2 { c.u8()? } else { 0 };
    let first = c.f64()?;
    knots.push(first);
    if clamped % 2 == 1 {
        while knots.len() <= order - 2 {
            knots.push(first);
        }
    }
    while knots.len() < count {
        knots.push(c.f64()?);
    }
    if clamped >= 2 {
        let last = *knots.last().unwrap();
        while knots.len() < knot_count {
            knots.push(last);
        }
    } else {
        while knots.len() < knot_count {
            knots.push(c.f64()?);
        }
    }
    knots.truncate(knot_count);
    let cv_dim = if rational > 0 { dim + 1 } else { dim };
    let mut cvs = Vec::with_capacity(count);
    for _ in 0..count {
        let mut v = [0.0; 4];
        for k in 0..cv_dim {
            v[if k == dim { 3 } else { k }] = c.f64()?;
        }
        if rational == 0 {
            v[3] = 1.0;
        } else {
            // as openNURBS reads them: the coordinates times the weight
            let w = v[3];
            for x in &mut v[..dim] {
                *x *= w;
            }
        }
        cvs.push(v);
    }
    if order == 2 && count == 2 && knots[0] > knots[1] {
        knots[0] = -knots[0];
        knots[1] = -knots[1];
    }
    nurbs_curve(order, &knots, cvs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use threedm_core::write::*;

    fn bytes_f64(v: &[f64]) -> Vec<u8> {
        f64s(v)
    }

    /// A Rhino 1 spline segment (`TCODE_LEGACY_SPL`): a straight one between two points.
    fn spline(w: &W, dim: u8, a: &[f64], b: &[f64]) -> Vec<u8> {
        let mut body = vec![dim, 0, 2];
        body.extend(2u16.to_le_bytes());
        body.extend([0, 0]);
        body.extend(bytes_f64(&vec![0.0; 2 * dim as usize]));
        body.extend(bytes_f64(&[0.0, 1.0])); // knots
        body.extend(bytes_f64(a));
        body.extend(bytes_f64(b));
        w.chunk(TCODE_LEGACY_SPL, &w.chunk(TCODE_LEGACY_SPLSTUFF, &body))
    }

    /// A Rhino 1 curve (`TCODE_LEGACY_CRVSTUFF`) of one straight segment.
    fn curve_stuff(w: &W, dim: u8, a: &[f64], b: &[f64]) -> Vec<u8> {
        let mut body = vec![dim, 0];
        body.extend(1i16.to_le_bytes());
        body.extend(bytes_f64(&vec![0.0; 2 * dim as usize]));
        body.extend(spline(w, dim, a, b));
        w.chunk(TCODE_LEGACY_CRVSTUFF, &body)
    }

    fn file() -> Vec<u8> {
        let w = W { big: false };
        let mut out = format!("3D Geometry File Format {:>8}", 1).into_bytes();
        // millimetres, a red layer
        out.extend(w.chunk(TCODE_UNIT_AND_TOLERANCES, &cat(&[&i32b(1), &i32b(2)])));
        let name = cat(&[&i32b(5), b"Walls"]);
        out.extend(w.chunk(TCODE_LAYER, &cat(&[&w.chunk(TCODE_LAYERNAME, &name), &w.short(TCODE_RGB, 0x0000FF), &w.short(TCODE_ENDOFTABLE, 0)])));
        let on_layer = w.short(TCODE_LAYERREF, 0);
        // a mesh of one triangle, in a box from (0,0,0) to (10,10,0)
        let mut mesh = cat(&[&i32b(3), &i32b(1), &i32b(0), &i32b(0), &bytes_f64(&[0.0, 0.0, 0.0, 10.0, 10.0, 0.0])]);
        for p in [[0u16, 0, 0], [65535, 0, 0], [0, 65535, 0]] {
            mesh.extend(p.iter().flat_map(|v| v.to_le_bytes()));
        }
        mesh.extend([0u16, 1, 2, 2].iter().flat_map(|v| v.to_le_bytes()));
        out.extend(w.chunk(TCODE_MESH_OBJECT, &cat(&[&w.chunk(TCODE_COMPRESSED_MESH_GEOMETRY, &mesh), &on_layer])));
        // a point with an arrow
        let mut xdata = cat(&[&i32b(16), &i32b(24)]);
        xdata.extend(b"RhAnnotateArrow\0");
        xdata.extend(bytes_f64(&[0.0, 0.0, 5.0]));
        out.extend(w.chunk(TCODE_RH_POINT, &cat(&[&bytes_f64(&[1.0, 2.0, 3.0]), &w.chunk(TCODE_XDATA, &xdata)])));
        // a curve from (0,0,0) to (3,4,0), named
        let named = w.chunk(TCODE_NAME, &cat(&[&i32b(4), b"Line"]));
        out.extend(w.chunk(TCODE_LEGACY_CRV, &cat(&[&named, &curve_stuff(&w, 3, &[0.0, 0.0, 0.0], &[3.0, 4.0, 0.0])])));
        // a face: the unit square, trimmed by its four sides
        let mut srf = vec![3, 0, 1, 1];
        srf.extend(1u16.to_le_bytes());
        srf.extend(1u16.to_le_bytes());
        srf.extend([0, 0, 0, 0, 0, 0]);
        srf.extend(bytes_f64(&[0.0; 6]));
        srf.extend(bytes_f64(&[0.0, 1.0, 0.0, 1.0]));
        srf.extend(bytes_f64(&[0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0]));
        let surface = w.chunk(TCODE_LEGACY_SRF, &w.chunk(TCODE_LEGACY_SRFSTUFF, &srf));
        let corners = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let mut trims = Vec::new();
        for k in 0..4 {
            let (a, b) = (corners[k], corners[(k + 1) % 4]);
            let mut t = vec![1u8];
            t.extend(cat(&[&i32b(0), &i32b(0), &i32b(0), &bytes_f64(&[0.0, 0.0])]));
            t.extend(w.chunk(TCODE_LEGACY_CRV, &curve_stuff(&w, 2, &a, &b)));
            t.extend(w.chunk(TCODE_LEGACY_CRV, &curve_stuff(&w, 3, &[a[0], a[1], 0.0], &[b[0], b[1], 0.0])));
            trims.extend(w.chunk(TCODE_LEGACY_TRM, &w.chunk(TCODE_LEGACY_TRMSTUFF, &t)));
        }
        let lp = w.chunk(TCODE_LEGACY_BND, &w.chunk(TCODE_LEGACY_BNDSTUFF, &cat(&[&i32b(4), &i32b(0), &bytes_f64(&[0.0, 0.0, 1.0, 1.0]), &trims])));
        let face = cat(&[&i32b(0), &i32b(0), &i32b(3), &bytes_f64(&[0.0, 0.0, 0.0, 1.0, 1.0, 0.0]), &i32b(0), &surface, &lp]);
        out.extend(w.chunk(TCODE_LEGACY_FAC, &cat(&[&on_layer, &w.chunk(TCODE_LEGACY_FACSTUFF, &face)])));
        out
    }

    #[test]
    fn rhino1_files_read_their_layers_meshes_points_curves_and_faces() {
        let data = file();
        let file = crate::read(&data).unwrap();
        assert_eq!((file.unit, file.layers.len(), file.objects.len()), ("millimetres", 1, 4));
        assert_eq!((file.layers[0].name.as_str(), file.layers[0].color), ("Walls", [255, 0, 0, 255]));
        assert!(file.damaged.is_empty(), "{:?}", file.damaged);
        let Geometry::Meshes(m, "Mesh") = &file.objects[0].geometry else { panic!("no mesh") };
        assert_eq!(m[0].positions, [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0]]);
        let Geometry::Lines(arrow, "Curve") = &file.objects[1].geometry else { panic!("no arrow") };
        assert_eq!(arrow[0], [DVec3::new(1.0, 2.0, 3.0), DVec3::new(1.0, 2.0, 8.0)]);
        let Geometry::Lines(line, "Curve") = &file.objects[2].geometry else { panic!("no curve") };
        assert_eq!((line[0][0], *line[0].last().unwrap(), file.objects[2].attributes.name.as_str()), (DVec3::ZERO, DVec3::new(3.0, 4.0, 0.0), "Line"));
        let Geometry::Meshes(faces, "Surface") = &file.objects[3].geometry else { panic!("no face: {:?}", std::mem::discriminant(&file.objects[3].geometry)) };
        let p = |m: &Mesh, i: u32| DVec3::from(m.positions[i as usize].map(f64::from));
        let area: f64 = faces.iter().flat_map(|m| m.faces.iter().map(move |f| 0.5 * (p(m, f[1]) - p(m, f[0])).cross(p(m, f[2]) - p(m, f[0])).length())).sum();
        assert!((area - 1.0).abs() < 1e-9, "{area}");
        // ids of their own, one each
        let ids: std::collections::HashSet<_> = file.objects.iter().map(|o| o.attributes.id).collect();
        assert_eq!(ids.len(), 4);
    }
}
