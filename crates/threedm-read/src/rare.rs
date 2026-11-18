//! The rarer object classes: cages (`ON_NurbsCage`, drawn as their control net and their six
//! faces meshed as NURBS surfaces), morph controls (`ON_MorphControl`: their curve, surface or
//! cage as lines, with their parameters), curves along polysurface edges (`ON_PolyEdgeCurve`,
//! their segments as stored and drawn along the edges they name), curves on surfaces
//! (`ON_CurveOnSurface`) and viewports (`ON_Viewport`, their frustum). Point grids and curve,
//! surface and offset surface proxies cannot be stored (openNURBS writes nothing of them); when
//! a file names one it is left out with that reason.

use glam::DVec3;
use threedm_core::*;
use threedm_model::{File, Geometry, Mesh, MorphControl, NurbsCage, PolyEdgeSegment};

use crate::tessellate;

pub(crate) const ON_NURBS_CAGE: Uuid = Uuid::parse("06936AFB-3D3C-41ac-BF70-C9319FA480A1");
pub(crate) const ON_MORPH_CONTROL: Uuid = Uuid::parse("D379E6D8-7C31-4407-A913-E3B7040D034A");
pub(crate) const ON_POLY_EDGE_CURVE: Uuid = Uuid::parse("39FF3DD3-FE0F-4807-9D59-185F0D73C0E4");
const ON_POLY_EDGE_SEGMENT: Uuid = Uuid::parse("42F47A87-5B1B-4e31-AB87-4639D78325D6");
const ON_CURVE_ON_SURFACE: Uuid = Uuid::parse("4ED7D4D8-E947-11d3-BFE5-0010830122F0");
const ON_VIEWPORT: Uuid = Uuid::parse("D66E5CCF-EA39-11d3-BFE5-0010830122F0");
/// Classes openNURBS does not write (their `Write` returns false).
const ON_POINT_GRID: Uuid = Uuid::parse("4ED7D4E5-E947-11d3-BFE5-0010830122F0");
const ON_CURVE_PROXY: Uuid = Uuid::parse("4ED7D4D9-E947-11d3-BFE5-0010830122F0");
const ON_SURFACE_PROXY: Uuid = Uuid::parse("4ED7D4E2-E947-11d3-BFE5-0010830122F0");
const ON_OFFSET_SURFACE: Uuid = Uuid::parse("00C61749-D430-4ecc-83A8-29130A20CF9C");

/// At most this many control points in a cage (a damaged count cannot allocate more).
const MAX_CAGE_POINTS: usize = 1 << 20;
/// `ON_COMPONENT_INDEX::brep_edge`.
const BREP_EDGE: i32 = 2;

/// The geometry of an object of a rare class; `None` when the class is none of them.
pub(crate) fn geometry<'a>(class: Uuid, c: &mut Cur<'_>, vertices: &mut usize) -> Option<R<Geometry<'a>>> {
    Some(if class == ON_NURBS_CAGE {
        read_cage(c).and_then(|cage| cage_drawing(&cage, vertices))
    } else if class == ON_MORPH_CONTROL {
        read_morph_control(c).map(|m| Geometry::Lines(morph_lines(&m), "Morph control"))
    } else if class == ON_POLY_EDGE_CURVE {
        // drawn once the polysurfaces they run along are read (`resolve_poly_edges`)
        read_poly_edge(c).map(|_| Geometry::Skipped("curves along polysurface edges not in the file"))
    } else if class == ON_CURVE_ON_SURFACE {
        curve_on_surface(c).map(|line| Geometry::Lines(vec![line], "Curve"))
    } else if class == ON_VIEWPORT {
        viewport(c).map(|lines| Geometry::Lines(lines, "Viewport"))
    } else if class == ON_POINT_GRID {
        Ok(Geometry::Skipped("point grids (openNURBS stores none)"))
    } else if [ON_CURVE_PROXY, ON_SURFACE_PROXY, ON_OFFSET_SURFACE].contains(&class) {
        Ok(Geometry::Skipped("curve and surface proxies (openNURBS stores none)"))
    } else {
        return None;
    })
}

/// The parameters of a cage, morph control or curve along edges, kept with the file.
pub(crate) fn extras(class: Uuid, mut c: Cur<'_>, index: usize, file: &mut File<'_>) {
    if class == ON_NURBS_CAGE {
        if let Ok(cage) = read_cage(&mut c) {
            file.cages.push((index, cage));
        }
    } else if class == ON_MORPH_CONTROL {
        if let Ok(mut m) = read_morph_control(&mut c) {
            m.object = index;
            file.morph_controls.push(m);
        }
    } else if class == ON_POLY_EDGE_CURVE {
        if let Ok(segments) = read_poly_edge(&mut c) {
            file.poly_edges.push((index, segments));
        }
    }
}

fn read_doubles(c: &mut Cur<'_>, n: usize) -> R<Vec<f64>> {
    if n.saturating_mul(8) > c.left() {
        return err("more numbers than their record holds");
    }
    (0..n).map(|_| c.f64()).collect()
}

/// Control points of `dim` coordinates (and a weight when rational) as homogeneous points.
fn read_cvs(c: &mut Cur<'_>, n: usize, dim: usize, rational: bool) -> R<Vec<[f64; 4]>> {
    let size = dim + rational as usize;
    if n.saturating_mul(8 * size) > c.left() || n > MAX_CAGE_POINTS {
        return err("more control points than their record holds");
    }
    (0..n)
        .map(|_| {
            let v = read_doubles(c, size)?;
            let mut h = [0.0, 0.0, 0.0, 1.0];
            let d = dim.min(3);
            h[..d].copy_from_slice(&v[..d]);
            if rational {
                h[3] = v[dim];
            }
            Ok(h)
        })
        .collect()
}

/// `ON_NurbsCage::Read`: in a chunk (1.0), dimension, rational, orders and control point
/// counts of the three directions, their knots (order + count − 2 each, no count stored), then
/// the control points, the third direction fastest.
fn read_cage(c: &mut Cur<'_>) -> R<NurbsCage> {
    let (mut b, major, _) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    if major != 1 {
        return err("a cage of an unknown version");
    }
    let dim = b.i32()?;
    let rational = b.i32()? != 0;
    let order = [b.i32()?, b.i32()?, b.i32()?].map(|v| v.max(0) as usize);
    let count = [b.i32()?, b.i32()?, b.i32()?].map(|v| v.max(0) as usize);
    if !(1..=3).contains(&dim) || (0..3).any(|d| order[d] < 2 || count[d] < order[d] || count[d] > MAX_CAGE_POINTS) {
        return err("a cage of an unknown form");
    }
    let mut knots: [Vec<f64>; 3] = Default::default();
    for d in 0..3 {
        knots[d] = read_doubles(&mut b, order[d] + count[d] - 2)?;
    }
    let total = count.iter().try_fold(1usize, |a, &n| a.checked_mul(n)).filter(|&n| n <= MAX_CAGE_POINTS).ok_or("Rhino 3dm: a cage with too many control points")?;
    let points = read_cvs(&mut b, total, dim as usize, rational)?;
    Ok(NurbsCage { order, count, knots, points })
}

/// `ON_NurbsCurve::Read`'s data as a cage of one row: version 1, dimension, rational, order,
/// count, two reserved ints and a bounding box, then knots and control points, each with a
/// count.
fn read_nurbs_curve(c: &mut Cur<'_>) -> R<NurbsCage> {
    let (major, _) = c.version()?;
    if major != 1 {
        return err("a NURBS curve of an unknown version");
    }
    let dim = c.i32()?.max(0) as usize;
    let rational = c.i32()? != 0;
    let order = c.i32()?.max(0) as usize;
    let count = c.i32()?.max(0) as usize;
    c.take(8 + 48)?;
    let k = c.count(8)?;
    let knots = read_doubles(c, k)?;
    let n = c.count(8)?;
    if !(1..=3).contains(&dim) || order < 2 || count < order || n != count || k != order + count - 2 {
        return err("a NURBS curve of an unknown form");
    }
    let points = read_cvs(c, n, dim, rational)?;
    Ok(NurbsCage { order: [order, 1, 1], count: [count, 1, 1], knots: [knots, Vec::new(), Vec::new()], points })
}

/// `ON_NurbsSurface::Read`'s data as a cage of one layer.
fn read_nurbs_surface(c: &mut Cur<'_>) -> R<NurbsCage> {
    let (major, _) = c.version()?;
    if major != 1 {
        return err("a NURBS surface of an unknown version");
    }
    let dim = c.i32()?.max(0) as usize;
    let rational = c.i32()? != 0;
    let order = [c.i32()?.max(0) as usize, c.i32()?.max(0) as usize];
    let count = [c.i32()?.max(0) as usize, c.i32()?.max(0) as usize];
    c.take(8 + 48)?;
    if !(1..=3).contains(&dim) || (0..2).any(|d| order[d] < 2 || count[d] < order[d]) {
        return err("a NURBS surface of an unknown form");
    }
    let mut knots = [Vec::new(), Vec::new(), Vec::new()];
    for d in 0..2 {
        let n = c.count(8)?;
        if n != order[d] + count[d] - 2 {
            return err("a NURBS surface of an unknown form");
        }
        knots[d] = read_doubles(c, n)?;
    }
    let n = c.count(8)?;
    if Some(n) != count[0].checked_mul(count[1]) {
        return err("a NURBS surface with the wrong number of control points");
    }
    let points = read_cvs(c, n, dim, rational)?;
    Ok(NurbsCage { order: [order[0], order[1], 1], count: [count[0], count[1], 1], knots, points })
}

/// `ON_MorphControl::Read`: in a chunk, of version 1 a cage, its captives and its start
/// transform; of version 2 the kind (1 curve, 2 surface, 3 cage), the start (a curve, surface
/// or transform) and the end (a curve, surface or cage) each in a chunk, the captives
/// (`ON_UuidList`), the localizers in a chunk, and from 2.1 the space morph tolerance, quick
/// preview and preserve structure.
fn read_morph_control(c: &mut Cur<'_>) -> R<MorphControl> {
    let (mut b, major, minor) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    let mut m = MorphControl {
        object: 0,
        kind: 3,
        control: NurbsCage::default(),
        start: None,
        cage_xform: None,
        captives: Vec::new(),
        localizers: 0,
        tolerance: 0.0,
        quick_preview: false,
        preserve_structure: false,
    };
    match major {
        1 => {
            m.control = read_cage(&mut b)?;
            m.captives = uuid_list(&mut b)?;
            m.cage_xform = Some(b.xform()?);
        }
        2 => {
            m.kind = b.i32()?;
            let (mut start, _, _) = b.versioned(TCODE_ANONYMOUS_CHUNK)?;
            match m.kind {
                1 => m.start = Some(read_nurbs_curve(&mut start)?),
                2 => m.start = Some(read_nurbs_surface(&mut start)?),
                3 => m.cage_xform = Some(start.xform()?),
                _ => return err("a morph control of an unknown kind"),
            }
            let (mut end, _, _) = b.versioned(TCODE_ANONYMOUS_CHUNK)?;
            m.control = match m.kind {
                1 => read_nurbs_curve(&mut end)?,
                2 => read_nurbs_surface(&mut end)?,
                _ => read_cage(&mut end)?,
            };
            m.captives = uuid_list(&mut b)?;
            let (mut localizers, _, _) = b.versioned(TCODE_ANONYMOUS_CHUNK)?;
            m.localizers = localizers.i32()?.max(0) as usize;
            if minor >= 1 {
                m.tolerance = b.f64()?;
                m.quick_preview = b.bool()?;
                m.preserve_structure = b.bool()?;
            }
        }
        _ => return err("a morph control of an unknown version"),
    }
    Ok(m)
}

/// `ON_UuidList::Read`: in a chunk (1.0), a count and the ids.
fn uuid_list(c: &mut Cur<'_>) -> R<Vec<Uuid>> {
    let (mut b, _, _) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    let n = b.count(16)?;
    (0..n).map(|_| b.uuid()).collect()
}

/// A control point as a point.
fn point(h: &[f64; 4]) -> DVec3 {
    let w = if h[3] != 0.0 { h[3] } else { 1.0 };
    DVec3::new(h[0] / w, h[1] / w, h[2] / w)
}

/// The control net of a cage (or of a curve or surface as one): a polyline along each
/// direction through every row of control points.
fn net(cage: &NurbsCage) -> Vec<Vec<DVec3>> {
    let [a, b, c] = cage.count;
    let at = |i: usize, j: usize, k: usize| point(&cage.points[(i * b + j) * c + k]);
    let mut lines = Vec::new();
    if a > 1 {
        for j in 0..b {
            for k in 0..c {
                lines.push((0..a).map(|i| at(i, j, k)).collect());
            }
        }
    }
    if b > 1 {
        for i in 0..a {
            for k in 0..c {
                lines.push((0..b).map(|j| at(i, j, k)).collect());
            }
        }
    }
    if c > 1 {
        for i in 0..a {
            for j in 0..b {
                lines.push((0..c).map(|k| at(i, j, k)).collect());
            }
        }
    }
    lines
}

/// The six faces of a cage, each a NURBS surface of its boundary control points, meshed as the
/// surfaces of polysurfaces are; the control net as lines.
fn cage_drawing<'a>(cage: &NurbsCage, vertices: &mut usize) -> R<Geometry<'a>> {
    let [a, b, c] = cage.count;
    let at = |i: usize, j: usize, k: usize| cage.points[(i * b + j) * c + k];
    let mut meshes: Vec<Mesh> = Vec::new();
    // each face: the two directions it spans, and its control points row by row
    type Face<'f> = ([usize; 2], Box<dyn Fn(usize, usize) -> [f64; 4] + 'f>);
    let faces: [Face<'_>; 6] = [
        ([1, 2], Box::new(|u, v| at(0, u, v))),
        ([1, 2], Box::new(|u, v| at(a - 1, u, v))),
        ([0, 2], Box::new(|u, v| at(u, 0, v))),
        ([0, 2], Box::new(|u, v| at(u, b - 1, v))),
        ([0, 1], Box::new(|u, v| at(u, v, 0))),
        ([0, 1], Box::new(|u, v| at(u, v, c - 1))),
    ];
    for ([du, dv], cv) in faces {
        let count = [cage.count[du], cage.count[dv]];
        let cvs = (0..count[0]).flat_map(|u| (0..count[1]).map(move |v| (u, v))).map(|(u, v)| cv(u, v)).collect();
        let surface = tessellate::nurbs_surface([cage.order[du], cage.order[dv]], count, [&cage.knots[du], &cage.knots[dv]], cvs)?;
        match tessellate::surface_mesh(&surface, vertices) {
            Ok(Some(mut mesh)) => {
                mesh.computed = true;
                meshes.push(mesh);
            }
            Err(e) if e == crate::TOO_MANY_VERTICES => return Err(e),
            _ => {}
        }
    }
    Ok(Geometry::Drawing { lines: net(cage), meshes, kind: "Cage" })
}

/// A morph control as lines: a curve control as its curve and control polygon, a surface or
/// cage control as its control net.
fn morph_lines(m: &MorphControl) -> Vec<Vec<DVec3>> {
    let mut lines = net(&m.control);
    if m.kind == 1 {
        if let Ok(curve) = tessellate::nurbs_curve(m.control.order[0], &m.control.knots[0], m.control.points.clone()) {
            lines.insert(0, tessellate::polyline(&curve));
        }
    }
    lines
}

/// `ON_PolyEdgeCurve`, written as `ON_PolyCurve`: version, segment count, two reserved ints, a
/// bounding box, the segment parameters, then its segments (`ON_PolyEdgeSegment`), each an
/// object: in a chunk, the object and component it runs along, the edge and trim domains,
/// whether it is reversed, its domain and the part of the edge it uses.
fn read_poly_edge(c: &mut Cur<'_>) -> R<Vec<PolyEdgeSegment>> {
    c.version()?;
    let count = c.count(4)?;
    c.take(8 + 48)?;
    let n = c.count(8)?;
    read_doubles(c, n)?;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let Some(o) = c.object()? else { continue };
        if o.class != ON_POLY_EDGE_SEGMENT {
            return err("a segment of an edge curve of another class");
        }
        let mut d = o.data;
        let (mut b, major, _) = d.versioned(TCODE_ANONYMOUS_CHUNK)?;
        if major != 1 {
            return err("an edge curve segment of an unknown version");
        }
        let object = b.uuid()?;
        let component = (b.i32()?, b.i32()?);
        let interval = |b: &mut Cur<'_>| -> R<[f64; 2]> { Ok([b.f64()?, b.f64()?]) };
        let edge_domain = interval(&mut b)?;
        let trim_domain = interval(&mut b)?;
        let reversed = b.bool()?;
        let domain = interval(&mut b)?;
        let curve_domain = interval(&mut b)?;
        out.push(PolyEdgeSegment { object, component, edge_domain, trim_domain, reversed, domain, curve_domain });
    }
    Ok(out)
}

/// Draws the curves along polysurface edges: each segment the part of its edge it uses, from
/// the polysurface of that id in the object table (`objects`, the table's records).
pub(crate) fn resolve_poly_edges(file: &mut File<'_>, mut tables: Cur<'_>) {
    if file.poly_edges.is_empty() {
        return;
    }
    let mut objects = None;
    while tables.left() > 0 {
        match tables.chunk() {
            Ok(chunk) if chunk.typecode == TCODE_OBJECT_TABLE => {
                let mut body = chunk.body;
                body.v = file.versions;
                objects = Some(body);
                break;
            }
            Ok(chunk) if chunk.typecode != TCODE_ENDOFFILE => {}
            _ => break,
        }
    }
    let Some(objects) = objects else { return };
    let wanted: Vec<Uuid> = file.poly_edges.iter().flat_map(|(_, s)| s.iter().map(|s| s.object)).collect();
    // the 3d curve and edge chunks of each polysurface asked for
    let mut breps: Vec<(Uuid, Cur<'_>, Cur<'_>)> = Vec::new();
    let mut t = objects;
    while t.left() > 0 {
        let Ok(chunk) = t.chunk() else { break };
        if chunk.typecode == TCODE_ENDOFTABLE {
            break;
        }
        if chunk.typecode != TCODE_OBJECT_RECORD {
            continue;
        }
        let Some((id, curves3, edges)) = brep_record(chunk.body) else { continue };
        if wanted.contains(&id) {
            breps.push((id, curves3, edges));
        }
    }
    for (index, segments) in &file.poly_edges {
        let mut line: Vec<DVec3> = Vec::new();
        for s in segments {
            let Some((_, curves3, edges)) = breps.iter().find(|b| b.0 == s.object) else { continue };
            if s.component.0 != BREP_EDGE {
                continue;
            }
            let Some((curve, proxy, edge_reversed)) = usize::try_from(s.component.1).ok().and_then(|e| tessellate::brep_edge(curves3.clone(), edges.clone(), e)) else { continue };
            // the edge's parameters are its curve's over the part it uses, flipped when it runs
            // backwards
            let on_curve = |t: f64| if edge_reversed { proxy[0] + proxy[1] - t } else { t };
            let (mut a, mut b) = (on_curve(s.curve_domain[0]), on_curve(s.curve_domain[1]));
            if s.reversed {
                std::mem::swap(&mut a, &mut b);
            }
            let part = tessellate::polyline_part(&curve, a, b);
            let skip = usize::from(line.last().zip(part.first()).is_some_and(|(p, q)| p.distance(*q) < 1e-9));
            line.extend(part.into_iter().skip(skip));
        }
        if line.len() >= 2 {
            if let Some(o) = file.objects.get_mut(*index) {
                o.geometry = Geometry::Lines(vec![line], "Curve");
            }
        }
    }
}

/// Of an object record holding a Brep (version 3): its id and its 3d curve and edge chunks.
fn brep_record(mut c: Cur<'_>) -> Option<(Uuid, Cur<'_>, Cur<'_>)> {
    let kind = c.chunk().ok()?;
    if kind.typecode != TCODE_OBJECT_RECORD_TYPE {
        return None;
    }
    let object = c.object().ok()??;
    if object.class != ON_BREP && !BREP_ALIASES.contains(&object.class) {
        return None;
    }
    let mut id = None;
    while c.left() > 0 {
        let chunk = c.chunk().ok()?;
        if chunk.typecode == TCODE_OBJECT_RECORD_ATTRIBUTES {
            id = crate::read_attributes(chunk.body).ok().map(|a| a.id);
            break;
        }
    }
    let mut d = object.data;
    if d.version().ok()?.0 != 3 {
        return None;
    }
    let _curves2 = d.chunk().ok()?;
    let curves3 = d.chunk().ok()?.body;
    let _surfaces = d.chunk().ok()?;
    let _vertices = d.chunk().ok()?;
    let edges = d.chunk().ok()?.body;
    Some((id?, curves3, edges))
}

/// `ON_CurveOnSurface::Read`: its 2d curve, whether it has a 3d curve and that curve, and its
/// surface, each an object. Drawn by its 3d curve, else its 2d curve on the surface.
fn curve_on_surface(c: &mut Cur<'_>) -> R<Vec<DVec3>> {
    let Some(c2) = tessellate::read_curve_object(c, 0)? else { return err("a curve on a surface without its curve") };
    let c3 = if c.i32()? != 0 { tessellate::read_curve_object(c, 0)? } else { None };
    if let Some(c3) = c3 {
        return Ok(tessellate::polyline(&c3));
    }
    let Some(surface) = tessellate::read_surface_object(c)? else { return err("a curve on a surface without its surface") };
    // the 2d curve's points, each piece split in eight, mapped onto the surface
    let uv = tessellate::polyline(&c2);
    let mut out = Vec::with_capacity(uv.len() * 8);
    for (k, w) in uv.windows(2).enumerate() {
        for s in usize::from(k > 0)..=8 {
            let p = w[0].lerp(w[1], s as f64 / 8.0);
            out.push(surface.point(p.x, p.y));
        }
    }
    Ok(out)
}

/// `ON_Viewport::Read` (version 1): its frustum as lines: the near and far rectangles and the
/// edges between them, in world coordinates from the camera's frame.
fn viewport(c: &mut Cur<'_>) -> R<Vec<Vec<DVec3>>> {
    let (major, _) = c.version()?;
    if major != 1 {
        return err("a viewport of an unknown version");
    }
    c.take(12)?; // valid camera, frustum, port
    let projection = c.i32()?;
    let mut p = || -> R<DVec3> { Ok(DVec3::new(c.f64()?, c.f64()?, c.f64()?)) };
    let location = p()?;
    let (_direction, _up) = (p()?, p()?);
    let (x, y, z) = (p()?, p()?, p()?);
    let f = read_doubles(c, 6)?;
    let (left, right, bottom, top, near, far) = (f[0], f[1], f[2], f[3], f[4], f[5]);
    if f.iter().any(|v| !v.is_finite()) || far <= near {
        return err("a viewport without a frustum");
    }
    let perspective = projection == 2 && near > 0.0;
    let rect = |d: f64| {
        let s = if perspective { d / near } else { 1.0 };
        let at = |u: f64, v: f64| location + s * u * x + s * v * y - d * z;
        vec![at(left, bottom), at(right, bottom), at(right, top), at(left, top), at(left, bottom)]
    };
    let (n, f) = (rect(near), rect(far));
    let mut lines = vec![n.clone(), f.clone()];
    lines.extend((0..4).map(|k| vec![n[k], f[k]]));
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use threedm_core::write::*;

    fn cur(data: &[u8]) -> Cur<'_> {
        Cur { data, pos: 0, v: Versions { archive: 90, opennurbs: 0 } }
    }

    /// A cage of degree 1 in every direction over the unit cube, 2×2×2 control points.
    fn cube(w: &W) -> Vec<u8> {
        let mut body = cat(&[&i32b(3), &i32b(0), &i32b(2), &i32b(2), &i32b(2), &i32b(2), &i32b(2), &i32b(2)]);
        for _ in 0..3 {
            body.extend(f64s(&[0.0, 1.0]));
        }
        for i in 0..2 {
            for j in 0..2 {
                for k in 0..2 {
                    body.extend(f64s(&[i as f64, j as f64, k as f64]));
                }
            }
        }
        w.versioned(TCODE_ANONYMOUS_CHUNK, 1, 0, &body)
    }

    #[test]
    fn a_cage_is_drawn_as_its_net_and_faces() {
        let w = W { big: true };
        let data = cube(&w);
        let mut vertices = 0;
        let Some(Ok(Geometry::Drawing { lines, meshes, kind })) = geometry(ON_NURBS_CAGE, &mut cur(&data), &mut vertices) else { panic!("no cage") };
        assert_eq!(kind, "Cage");
        assert_eq!(lines.len(), 12);
        assert_eq!(meshes.len(), 6);
        let area: f64 = meshes
            .iter()
            .flat_map(|m| m.faces.iter().map(move |f| (m, f)))
            .map(|(m, f)| {
                let p = |i: u32| DVec3::from(m.positions[i as usize].map(f64::from));
                let tri = |a: u32, b: u32, c: u32| 0.5 * (p(b) - p(a)).cross(p(c) - p(a)).length();
                tri(f[0], f[1], f[2]) + if f[2] != f[3] { tri(f[0], f[2], f[3]) } else { 0.0 }
            })
            .sum();
        assert!((area - 6.0).abs() < 1e-6, "{area}");
    }

    #[test]
    fn a_viewport_is_drawn_as_its_frustum() {
        let mut body = vec![0x11];
        body.extend(cat(&[&i32b(1), &i32b(1), &i32b(1), &i32b(2)]));
        body.extend(f64s(&[0.0, 0.0, 10.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]));
        body.extend(f64s(&[-1.0, 1.0, -1.0, 1.0, 1.0, 5.0]));
        let mut vertices = 0;
        let Some(Ok(Geometry::Lines(lines, "Viewport"))) = geometry(ON_VIEWPORT, &mut cur(&body), &mut vertices) else { panic!("no viewport") };
        assert_eq!(lines.len(), 6);
        assert_eq!(lines[1][0], DVec3::new(-5.0, -5.0, 5.0));
    }

    /// `ON_LineCurve` data from `a` to `b` over [0, 10].
    fn line(a: [f64; 3], b: [f64; 3]) -> Vec<u8> {
        cat(&[&[0x10], &f64s(&a), &f64s(&b), &f64s(&[0.0, 10.0]), &i32b(3)])
    }

    /// A curve along the edge of a polysurface in the same file is drawn along the part of the
    /// edge it uses; a curve on a surface by its 3d curve.
    #[test]
    fn curves_along_edges_follow_their_edge() {
        let w = W { big: true };
        let (brep_id, curve_id) = (id(1), id(2));
        let line_class = Uuid::parse("4ED7D4DB-E947-11d3-BFE5-0010830122F0");
        // a Brep (3.2) of one 3d curve and one edge using all of it; its other arrays empty
        let empty = |v: u8| w.chunk(TCODE_ANONYMOUS_CHUNK, &cat(&[&[v], &i32b(0)]));
        let curves3 = w.chunk(TCODE_ANONYMOUS_CHUNK, &cat(&[&[0x10], &i32b(1), &i32b(1), &w.object(line_class, &line([0.0; 3], [10.0, 0.0, 0.0]), &[])]));
        let edge = cat(&[&i32b(0), &i32b(0), &i32b(0), &f64s(&[0.0, 10.0]), &i32b(-1), &i32b(-1), &i32b(0), &f64s(&[0.001]), &f64s(&[0.0, 10.0])]);
        let edges = w.chunk(TCODE_ANONYMOUS_CHUNK, &cat(&[&[0x10], &i32b(1), &edge]));
        let brep = cat(&[&[0x32], &empty(0x10), &curves3, &empty(0x10), &empty(0x10), &edges, &empty(0x11), &empty(0x10), &empty(0x10), &f64s(&[0.0; 6])]);
        // the curve: one segment along edge 0, from 2 to 8
        let segment = w.versioned(TCODE_ANONYMOUS_CHUNK, 1, 0, &cat(&[&brep_id.0, &i32b(BREP_EDGE), &i32b(0), &f64s(&[0.0, 10.0, 0.0, 1.0]), &[0], &f64s(&[0.0, 6.0, 2.0, 8.0])]));
        let poly = cat(&[&[0x10], &i32b(1), &i32b(0), &i32b(0), &f64s(&[0.0; 6]), &i32b(2), &f64s(&[0.0, 6.0]), &w.object(ON_POLY_EDGE_SEGMENT, &segment, &[])]);
        // a curve on a surface: its 2d curve, its 3d curve, and (unread) no surface
        let on_surface = cat(&[&w.object(line_class, &line([0.0; 3], [1.0, 1.0, 0.0]), &[]), &i32b(1), &w.object(line_class, &line([0.0, 0.0, 5.0], [3.0, 4.0, 5.0]), &[])]);
        let record = |class: Uuid, data: &[u8], id: Uuid| w.record(4, &w.object(class, data, &[]), &attributes(id, 0, &[]), &[]);
        let objects = w.table(TCODE_OBJECT_TABLE, &[record(ON_POLY_EDGE_CURVE, &poly, curve_id), record(ON_BREP, &brep, brep_id), record(ON_CURVE_ON_SURFACE, &on_surface, id(3))]);
        let bytes = w.file(60, 2_600_000_000, 2, &[objects]);
        let file = crate::read(&bytes).unwrap();
        let Geometry::Lines(lines, "Curve") = &file.objects[0].geometry else { panic!("{:?}", file.objects[0].geometry) };
        let (first, last) = (lines[0][0], lines[0][lines[0].len() - 1]);
        assert!(first.distance(DVec3::new(2.0, 0.0, 0.0)) < 1e-9 && last.distance(DVec3::new(8.0, 0.0, 0.0)) < 1e-9, "{first} {last}");
        assert_eq!(file.poly_edges[0].1[0].object, brep_id);
        let Geometry::Lines(lines, "Curve") = &file.objects[2].geometry else { panic!("{:?}", file.objects[2].geometry) };
        assert_eq!(lines[0], [DVec3::new(0.0, 0.0, 5.0), DVec3::new(3.0, 4.0, 5.0)]);
    }
}
