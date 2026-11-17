//! Dimensions and leaders: linear (`ON_DimLinear`), angular (`ON_DimAngular`), radius and
//! diameter (`ON_DimRadial`), ordinate (`ON_DimOrdinate`) dimensions, centre marks
//! (`ON_Centermark`) and leaders (`ON_Leader`), read as openNURBS writes them and drawn as Rhino
//! draws them: extension lines, the dimension line or arc, arrowheads of the style's kind, and
//! the measured value (or the leader's text) set in the font, all at the style's sizes times its
//! model space scale. The value is formatted with the style's resolution, length factor, length
//! display, prefix and suffix, into the dimension's own text where it holds `<>`.

use super::drafting::{read_annotation, set_text, Annotation, DimStyle};
use super::{Cur, Mesh, Uuid, R, TCODE_ANONYMOUS_CHUNK};
use glam::{DVec2, DVec3};

pub(super) const ON_DIM_LINEAR: Uuid = Uuid::parse("E550882B-F44D-4154-A1EF-6E50CBBBF543");
pub(super) const ON_DIM_ANGULAR: Uuid = Uuid::parse("D417786B-F6CD-4F12-9E1F-063F414DBEB6");
pub(super) const ON_DIM_RADIAL: Uuid = Uuid::parse("FC749C2F-4C00-41FD-9840-26D94F047AD3");
pub(super) const ON_DIM_ORDINATE: Uuid = Uuid::parse("03124828-4C9B-4D28-9A82-664DDDE7A14F");
pub(super) const ON_CENTERMARK: Uuid = Uuid::parse("D46767BA-7E8F-4D9D-9A92-66050219A5B9");
pub(super) const ON_LEADER: Uuid = Uuid::parse("945BF594-6FF9-4F5C-BFC0-B3AF528F29D2");

/// At most this many points of a leader are drawn.
const MAX_LEADER_POINTS: usize = 1000;

/// Whether `class` is one of the dimensions or a leader.
pub(super) fn is_annotation(class: Uuid) -> bool {
    [ON_DIM_LINEAR, ON_DIM_ANGULAR, ON_DIM_RADIAL, ON_DIM_ORDINATE, ON_CENTERMARK, ON_LEADER].contains(&class)
}

/// What the dimension adds to the annotation (`ON_Dimension::Internal_ReadDimension`).
struct Head {
    use_default_text_point: bool,
    user_text_point: DVec2,
    flip_arrows: [bool; 2],
}

fn point2(c: &mut Cur<'_>) -> R<DVec2> {
    Ok(DVec2::new(c.f64()?, c.f64()?))
}

/// The dimension chunk at `c`: the annotation, then the dimension's own fields.
fn read_head(c: &mut Cur<'_>) -> R<(Annotation, Head)> {
    let (mut d, _, version) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    let annotation = read_annotation(&mut d)?;
    d.wstring()?; // user text (the annotation's text holds what is shown)
    d.f64()?; // obsolete text rotation
    let use_default_text_point = d.bool()?;
    let user_text_point = point2(&mut d)?;
    let flip_arrows = [d.bool()?, d.bool()?];
    d.i32()?; // arrow fit
    d.uuid()?; // the detail measured
    d.f64()?; // distance scale
    if version >= 1 {
        d.i32()?; // text fit
    }
    Ok((annotation, Head { use_default_text_point, user_text_point, flip_arrows }))
}

/// The lines and filled shapes (arrowheads, text) of an annotation, and its kind to show.
pub(super) struct Drawing {
    pub(super) lines: Vec<Vec<DVec3>>,
    pub(super) meshes: Vec<Mesh>,
    pub(super) kind: &'static str,
}

/// Reads a dimension, centre mark or leader of class `class` and draws it with its style
/// (`styles` by id, the first when its own is not found), in `font`. `meters` is the length of
/// a model unit, for the length display.
pub(super) fn read(class: Uuid, c: &mut Cur<'_>, styles: &[DimStyle], font: &[u8], meters: f64) -> R<Drawing> {
    let (mut outer, _, version) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    if class == ON_LEADER {
        if version < 1 {
            return super::err("a leader of an unknown version");
        }
        let a = read_annotation(&mut outer)?;
        let n = outer.count(16)?;
        let points = (0..n).map(|_| point2(&mut outer)).collect::<R<Vec<_>>>()?;
        let mut pen = Pen::new(&a, styles, font, meters);
        pen.leader(&a.text, &points);
        return Ok(pen.done("Leader"));
    }
    let (a, head) = read_head(&mut outer)?;
    let mut pen = Pen::new(&a, styles, font, meters);
    if class == ON_DIM_LINEAR {
        let def2 = point2(&mut outer)?;
        let dimline = point2(&mut outer)?;
        pen.linear(&a.text, &head, def2, dimline);
    } else if class == ON_DIM_ANGULAR {
        let vec1 = point2(&mut outer)?;
        let vec2 = point2(&mut outer)?;
        let offsets = [outer.f64()?, outer.f64()?];
        let dimline = point2(&mut outer)?;
        pen.angular(&a.text, &head, [vec1, vec2], offsets, dimline);
    } else if class == ON_DIM_RADIAL {
        let radius = point2(&mut outer)?;
        let dimline = point2(&mut outer)?;
        pen.radial(&a.text, a.kind == 3, radius, dimline);
    } else if class == ON_DIM_ORDINATE {
        let direction = outer.u32()?;
        let def = point2(&mut outer)?;
        let leader = point2(&mut outer)?;
        let kinks = [outer.f64()?, outer.f64()?];
        pen.ordinate(&a.text, direction, def, leader, kinks);
    } else if class == ON_CENTERMARK {
        outer.f64()?; // radius
        pen.centermark(DVec2::ZERO);
        return Ok(pen.done("Center mark"));
    } else {
        return super::err("not a dimension");
    }
    Ok(pen.done("Dimension"))
}

/// Draws in the plane of an annotation with its style.
struct Pen<'f> {
    origin: DVec3,
    x: DVec3,
    y: DVec3,
    style: DimStyle,
    /// The model space scale applied to every size.
    s: f64,
    font: &'f [u8],
    meters: f64,
    lines: Vec<Vec<DVec3>>,
    meshes: Vec<Mesh>,
}

impl<'f> Pen<'f> {
    fn new(a: &Annotation, styles: &[DimStyle], font: &'f [u8], meters: f64) -> Self {
        let style = a.own_style.clone().or_else(|| styles.iter().find(|d| d.id == a.style).or(styles.first()).cloned()).unwrap_or_default();
        let s = if a.allow_scaling { style.scale } else { 1.0 };
        Pen { origin: a.origin, x: a.x.normalize_or_zero(), y: a.y.normalize_or_zero(), style, s, font, meters, lines: Vec::new(), meshes: Vec::new() }
    }

    fn done(self, kind: &'static str) -> Drawing {
        Drawing { lines: self.lines, meshes: self.meshes, kind }
    }

    fn at(&self, p: DVec2) -> DVec3 {
        self.origin + p.x * self.x + p.y * self.y
    }

    fn line(&mut self, a: DVec2, b: DVec2) {
        if a.is_finite() && b.is_finite() && a.distance(b) > 1e-12 {
            self.lines.push(vec![self.at(a), self.at(b)]);
        }
    }

    fn polyline(&mut self, points: &[DVec2]) {
        if points.len() >= 2 && points.iter().all(|p| p.is_finite()) {
            self.lines.push(points.iter().map(|&p| self.at(p)).collect());
        }
    }

    /// A filled polygon (convex, in order) in the plane.
    fn fill(&mut self, points: &[DVec2]) {
        if points.len() < 3 || !points.iter().all(|p| p.is_finite()) {
            return;
        }
        let positions = points.iter().map(|&p| self.at(p).to_array().map(|v| v as f32)).collect();
        let faces: Vec<[u32; 4]> = (1..points.len() as u32 - 1).map(|k| [0, k, k + 1, k + 1]).collect();
        let triangles = faces.len();
        self.meshes.push(Mesh { positions, faces, triangles, ..Mesh::default() });
    }

    /// An arrowhead of `kind` (`ON_Arrowhead::arrow_type`), `size` long, its tip at `tip`, its
    /// body towards `body` (a direction).
    fn arrow(&mut self, kind: u32, tip: DVec2, body: DVec2, size: f64) {
        let b = body.normalize_or_zero();
        if b == DVec2::ZERO || size <= 0.0 {
            return;
        }
        let n = b.perp();
        let triangle = |half: f64| [tip, tip + b * size + n * size * half, tip + b * size - n * size * half];
        match kind {
            0 => {}
            3 => {
                // a dot on the tip
                let r = size / 2.0;
                let ring: Vec<DVec2> = (0..16).map(|k| tip + DVec2::from_angle(k as f64 * std::f64::consts::TAU / 16.0) * r).collect();
                self.fill(&ring);
            }
            4 => {
                // a tick across the tip, at 45 degrees
                let d = (b + n) * size / 2.0;
                self.line(tip - d, tip + d);
            }
            5 => self.fill(&triangle(0.5)),
            6 => {
                let [t, l, r] = triangle(0.25);
                self.line(l, t);
                self.line(t, r);
            }
            7 => self.fill(&[tip + n * size / 4.0, tip + b * size + n * size / 4.0, tip + b * size - n * size / 4.0, tip - n * size / 4.0]),
            8 => self.fill(&triangle(1.0 / 8.0)),
            9 => self.fill(&triangle(1.0 / 12.0)),
            // solid triangles, and blocks (drawn as such)
            _ => self.fill(&triangle(0.25)),
        }
    }

    /// The direction to read text along `along` (in the plane): turned over when it would read
    /// right to left or downwards in the model.
    fn readable(&self, along: DVec2) -> DVec2 {
        let along = along.normalize_or_zero();
        let world = along.x * self.x + along.y * self.y;
        if world.x < -1e-9 || (world.x.abs() <= 1e-9 && world.y < 0.0) {
            -along
        } else {
            along
        }
    }

    /// Sets `text` at `at`, reading along `along` (made readable), aligned `h` and `v` as in
    /// `set_text`; returns the extent of the text along its reading direction from `at`.
    fn text(&mut self, text: &str, at: DVec2, along: DVec2, h: u32, v: u32) -> Option<(f64, f64)> {
        let along = self.readable(along);
        if along == DVec2::ZERO || text.trim().is_empty() {
            return None;
        }
        let x = along.x * self.x + along.y * self.y;
        let up = along.perp();
        let y = up.x * self.x + up.y * self.y;
        let height = self.style.height * self.s;
        let mesh = set_text(text, self.at(at), x, y, height, h, v, self.font)?;
        let origin = self.at(at);
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for p in &mesh.positions {
            let t = (DVec3::from(p.map(f64::from)) - origin).dot(x);
            lo = lo.min(t);
            hi = hi.max(t);
        }
        self.meshes.push(mesh);
        Some((lo, hi))
    }

    /// The dimension's text with `value` where it holds `<>`.
    fn value_text(&self, text: &str, value: String) -> String {
        let text = if text.is_empty() { "<>" } else { text };
        text.replace("<>", &value)
    }

    /// A length in model units as the style shows it.
    fn length(&self, model: f64) -> String {
        let st = &self.style;
        let unit = match st.length_display {
            3 => Some(0.001),
            4 => Some(0.01),
            5 => Some(1.0),
            6 => Some(1000.0),
            1 | 7 => Some(0.0254),
            2 | 8 => Some(0.3048),
            _ => None,
        };
        let shown = model * st.length_factor * unit.map_or(1.0, |u| self.meters / u);
        format!("{}{:.*}{}", st.prefix, st.length_resolution as usize, shown, st.suffix)
    }

    fn angle(&self, radians: f64) -> String {
        format!("{:.*}\u{b0}", self.style.angle_resolution as usize, radians.to_degrees())
    }

    /// Text next to a line from `from` to `to`, on the side Rhino puts it: above (baseline a gap
    /// up), in it (centred on it, the line cut around it) or below.
    fn text_on_line(&mut self, text: &str, at: DVec2, along: DVec2, cut: Option<(DVec2, DVec2)>) {
        let gap = self.style.text_gap * self.s;
        let up = self.readable(along).perp();
        match self.style.text_location {
            1 => {
                let extent = self.text(text, at, along, 1, 3);
                if let (Some((lo, hi)), Some((a, b))) = (extent, cut) {
                    // the line, less the text and a gap on either side
                    let dir = self.readable(along);
                    let (t0, t1) = ((lo - gap), (hi + gap));
                    let ta = (a - at).dot(dir);
                    let tb = (b - at).dot(dir);
                    let (first, last, ta, tb) = if ta <= tb { (a, b, ta, tb) } else { (b, a, tb, ta) };
                    if t0 > ta {
                        self.line(first, at + dir * t0.min(tb));
                    }
                    if t1 < tb {
                        self.line(at + dir * t1.max(ta), last);
                    }
                    return;
                }
                if let Some((a, b)) = cut {
                    self.line(a, b);
                }
            }
            2 => {
                self.text(text, at - up * gap, along, 1, 0);
                if let Some((a, b)) = cut {
                    self.line(a, b);
                }
            }
            _ => {
                self.text(text, at + up * gap, along, 1, 5);
                if let Some((a, b)) = cut {
                    self.line(a, b);
                }
            }
        }
    }

    /// `ON_DimLinear`: the first point at the plane's origin, the second at `def2`, the
    /// dimension line through `dimline`, parallel to the plane's x axis.
    fn linear(&mut self, text: &str, head: &Head, def2: DVec2, dimline: DVec2) {
        let st = self.style.clone();
        let (eo, ee, de) = (st.ext_offset * self.s, st.ext_extension * self.s, st.dim_extension * self.s);
        let yd = dimline.y;
        if !st.suppress_extension[0] {
            let sign = if yd < 0.0 { -1.0 } else { 1.0 };
            self.line(DVec2::new(0.0, sign * eo), DVec2::new(0.0, yd + sign * ee));
        }
        if !st.suppress_extension[1] {
            let sign = if yd < def2.y { -1.0 } else { 1.0 };
            self.line(DVec2::new(def2.x, def2.y + sign * eo), DVec2::new(def2.x, yd + sign * ee));
        }
        let toward = if def2.x < 0.0 { -1.0 } else { 1.0 };
        let a = DVec2::new(-de * toward, yd);
        let b = DVec2::new(def2.x + de * toward, yd);
        let at = if head.use_default_text_point { DVec2::new(def2.x / 2.0, yd) } else { head.user_text_point };
        let shown = self.value_text(text, self.length(def2.x.abs()));
        self.text_on_line(&shown, at, DVec2::X, Some((a, b)));
        let size = st.arrow * self.s;
        let ends = [(DVec2::new(0.0, yd), DVec2::X * toward), (DVec2::new(def2.x, yd), -DVec2::X * toward)];
        for (k, (tip, body)) in ends.into_iter().enumerate() {
            if !st.suppress_arrow[k] {
                let body = if head.flip_arrows[k] { -body } else { body };
                self.arrow(st.arrows[k], tip, body, size);
            }
        }
    }

    /// `ON_DimAngular`: the angle at the plane's origin between `vectors`, the points measured
    /// `offsets` from the centre, the arc through `dimline`.
    fn angular(&mut self, text: &str, head: &Head, vectors: [DVec2; 2], offsets: [f64; 2], dimline: DVec2) {
        let st = self.style.clone();
        let r = dimline.length();
        let [v1, v2] = vectors.map(|v| v.normalize_or_zero());
        if r <= 0.0 || v1 == DVec2::ZERO || v2 == DVec2::ZERO {
            return;
        }
        let tau = std::f64::consts::TAU;
        let (a1, a2, ad) = (v1.y.atan2(v1.x), v2.y.atan2(v2.x), dimline.y.atan2(dimline.x));
        let mut start = a1;
        let mut span = (a2 - a1).rem_euclid(tau);
        if (ad - a1).rem_euclid(tau) > span {
            // the arc through the dimension line point runs the other way round
            start = a2;
            span = tau - span;
        }
        let n = ((span / (std::f64::consts::PI / 32.0)).ceil() as usize).max(8);
        let arc: Vec<DVec2> = (0..=n).map(|k| DVec2::from_angle(start + span * k as f64 / n as f64) * r).collect();
        self.polyline(&arc);
        let (eo, ee) = (st.ext_offset * self.s, st.ext_extension * self.s);
        for (k, v) in [v1, v2].into_iter().enumerate() {
            let d = offsets[k];
            if !st.suppress_extension[k] && d.is_finite() && (r - d).abs() > 1e-9 {
                let sign = (r - d).signum();
                self.line(v * (d + sign * eo), v * (r + sign * ee));
            }
        }
        // arrowheads at the ends of the arc, along it
        let size = st.arrow * self.s;
        let step = (size / r).min(span / 2.0);
        let ends = [(start, start + step), (start + span, start + span - step)];
        for (k, (tip, toward)) in ends.into_iter().enumerate() {
            if !st.suppress_arrow[k] {
                let tip = DVec2::from_angle(tip) * r;
                let body = DVec2::from_angle(toward) * r - tip;
                let body = if head.flip_arrows[k] { -body } else { body };
                self.arrow(st.arrows[k], tip, body, size);
            }
        }
        // the value at the middle of the arc, outside it
        let mid = start + span / 2.0;
        let radial = DVec2::from_angle(mid);
        let at = if head.use_default_text_point { radial * r } else { head.user_text_point };
        let along = radial.perp();
        let gap = st.text_gap * self.s;
        let up = self.readable(along).perp();
        let v = if up.dot(radial) >= 0.0 { 5 } else { 0 };
        let shown = self.value_text(text, self.angle(span));
        self.text(&shown, at + radial * gap, along, 1, v);
    }

    /// `ON_DimRadial`: the centre at the plane's origin, the arrow at `radius`, a line to the
    /// knee level with `dimline`, on to it, and the landing with the value beyond.
    fn radial(&mut self, text: &str, diameter: bool, radius: DVec2, dimline: DVec2) {
        let st = self.style.clone();
        let r = radius.length();
        let dir = radius.normalize_or_zero();
        if r <= 0.0 {
            return;
        }
        let knee = if dir.y.abs() > 1e-9 && dimline.y / dir.y > 0.0 { dir * (dimline.y / dir.y) } else { dimline };
        self.line(radius, knee);
        self.line(knee, dimline);
        let tail = if dimline.x - knee.x > -1e-9 { DVec2::X } else { -DVec2::X };
        let tail = if (dimline.x - knee.x).abs() < 1e-9 && dimline.x < radius.x { -DVec2::X } else { tail };
        let landing = st.landing * self.s;
        let end = dimline + tail * landing;
        self.line(dimline, end);
        let gap = st.text_gap * self.s;
        let shown = self.value_text(text, self.length(if diameter { 2.0 * r } else { r }));
        self.text(&shown, end + tail * gap, tail, if tail.x > 0.0 { 0 } else { 2 }, 3);
        if !st.suppress_arrow[0] {
            self.arrow(st.arrows[0], radius, knee - radius, st.arrow * self.s);
        }
        self.centermark(DVec2::ZERO);
    }

    /// A cross at `centre`, the style's centre mark size each way.
    fn centermark(&mut self, centre: DVec2) {
        let c = self.style.centermark * self.s;
        if c > 0.0 {
            self.line(centre - DVec2::X * c, centre + DVec2::X * c);
            self.line(centre - DVec2::Y * c, centre + DVec2::Y * c);
        }
    }

    /// `ON_DimOrdinate`: the x (`direction` 1) or y (2) of `def` from the plane's origin, on a
    /// leader from `def` to `leader` (bent at the kinks) and on past it as far as the text.
    fn ordinate(&mut self, text: &str, direction: u32, def: DVec2, leader: DVec2, kinks: [f64; 2]) {
        let st = self.style.clone();
        // the leader runs along y when x is measured, and along x when y is; Rhino goes by
        // which way the leader runs
        let run = leader - def;
        let along_x = if (run.x.abs() - run.y.abs()).abs() > 1e-9 { run.x.abs() > run.y.abs() } else { direction == 2 };
        let (value, along) = if along_x { (def.y, DVec2::X) } else { (def.x, DVec2::Y) };
        let across = along.perp();
        let run = (leader - def).dot(along);
        let sign = if run < 0.0 { -1.0 } else { 1.0 };
        let dir = along * sign;
        let start = def + dir * st.ext_offset * self.s;
        let shift = (leader - def).dot(across);
        if shift.abs() < 1e-9 {
            self.line(start, leader);
        } else {
            // bent: along the measured line, across, and on to the leader point
            let k1 = kinks[0].abs().max(0.0);
            let k2 = kinks[1].abs().max(0.0);
            let total = run.abs();
            let first = if k1 > 0.0 { k1.min(total) } else { total / 2.0 };
            let second = if k2 > 0.0 { k2.min(total - first) } else { 0.0 };
            let p1 = def + dir * first;
            let p2 = p1 + across * shift + dir * second;
            self.polyline(&[start, p1, p2, leader]);
        }
        let gap = st.text_gap * self.s;
        let shown = self.value_text(text, self.length(value.abs()));
        // the text along the extension past the leader point, a gap from its start
        let read = self.readable(dir);
        let probe = self.text(&shown, DVec2::ZERO, dir, 0, 5);
        let width = probe.map_or(0.0, |(lo, hi)| hi - lo);
        if probe.is_some() {
            self.meshes.pop();
        }
        let end = leader + dir * (width + 2.0 * gap);
        self.line(leader, end);
        let first = if read.dot(dir) > 0.0 { leader } else { end };
        let up = read.perp();
        self.text(&shown, first + read * gap + up * gap, dir, 0, 5);
    }

    /// `ON_Leader`: a line through `points` (in the plane) with an arrowhead at the first, the
    /// landing and the text at the last.
    fn leader(&mut self, text: &str, points: &[DVec2]) {
        let st = self.style.clone();
        let points: Vec<DVec2> = points.iter().copied().filter(|p| p.is_finite()).take(MAX_LEADER_POINTS).collect();
        if points.len() < 2 {
            return;
        }
        self.polyline(&points);
        self.arrow(st.leader_arrow_type, points[0], points[1] - points[0], st.leader_arrow * self.s);
        let last = points[points.len() - 1];
        let before = points[points.len() - 2];
        let tail = if last.x - before.x < -1e-9 { -DVec2::X } else { DVec2::X };
        let mut end = last;
        if st.has_landing {
            end = last + tail * st.landing * self.s;
            self.line(last, end);
        }
        let gap = st.text_gap * self.s;
        self.text(text, end + tail * gap, tail, if tail.x > 0.0 { 0 } else { 2 }, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pen<'f>(style: DimStyle, font: &'f [u8]) -> Pen<'f> {
        Pen { origin: DVec3::ZERO, x: DVec3::X, y: DVec3::Y, style, s: 1.0, font, meters: 0.001, lines: Vec::new(), meshes: Vec::new() }
    }

    fn style() -> DimStyle {
        DimStyle { height: 1.0, ext_extension: 0.5, ext_offset: 0.5, arrow: 1.0, leader_arrow: 1.0, text_gap: 0.25, length_resolution: 1, centermark: 3.5, landing: 3.5, ..DimStyle::default() }
    }

    #[test]
    fn lengths_show_as_the_style_says() {
        let mut st = style();
        let p = pen(st.clone(), &[]);
        assert_eq!(p.length(10.0), "10.0");
        st.length_resolution = 0;
        st.prefix = "L=".into();
        st.suffix = " mm".into();
        assert_eq!(pen(st.clone(), &[]).length(12.5), "L=12 mm");
        // metres shown for a model in millimetres
        st.length_display = 5;
        st.length_resolution = 2;
        assert_eq!(pen(st, &[]).length(1234.0), "L=1.23 mm");
        assert_eq!(p.angle(1.0), "57\u{b0}");
        assert_eq!(p.value_text("R<>", "4.0".into()), "R4.0");
    }

    /// A horizontal dimension of 10 with its line 3 up, as Rhino 9 draws it: extension lines
    /// from 0.5 to 3.5, the dimension line from 0 to 10, arrowheads 1 long and 0.5 wide.
    #[test]
    fn a_linear_dimension_has_its_lines_and_arrows() {
        let mut p = pen(style(), &[]);
        let head = Head { use_default_text_point: true, user_text_point: DVec2::ZERO, flip_arrows: [false; 2] };
        p.linear("<>", &head, DVec2::new(10.0, 0.0), DVec2::new(5.0, 3.0));
        let lines: Vec<[[f64; 2]; 2]> = p.lines.iter().map(|l| [[l[0].x, l[0].y], [l[1].x, l[1].y]]).collect();
        assert_eq!(lines, [[[0.0, 0.5], [0.0, 3.5]], [[10.0, 0.5], [10.0, 3.5]], [[0.0, 3.0], [10.0, 3.0]]]);
        // no font: the two arrowheads only
        assert_eq!(p.meshes.len(), 2);
        let tri: Vec<[f32; 3]> = p.meshes[0].positions.clone();
        assert_eq!(tri, [[0.0, 3.0, 0.0], [1.0, 3.25, 0.0], [1.0, 2.75, 0.0]]);
    }

    /// A radius as Rhino 9 draws it: from the arrow at the circle to the knee level with the
    /// dimension line point, on to it, the landing, and the centre mark.
    #[test]
    fn a_radius_bends_at_its_knee() {
        let mut p = pen(style(), &[]);
        let radius = DVec2::from_angle(0.8) * 4.0;
        p.radial("R<>", false, radius, DVec2::new(7.0, 6.0));
        let knee = p.lines[0][1];
        assert!((knee.x - 5.8273).abs() < 1e-3 && (knee.y - 6.0).abs() < 1e-9, "{knee}");
        let landing = &p.lines[2];
        assert!((landing[1].x - 10.5).abs() < 1e-9);
        assert_eq!(p.lines.len(), 5, "and the two lines of the centre mark");
    }
}
