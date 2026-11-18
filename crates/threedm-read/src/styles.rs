//! Linetypes and section styles, as openNURBS writes them.
//!
//! A linetype (`ON_Linetype`) is a pattern of dashes and gaps; from Rhino 8 also a width (with
//! its unit), the cap of its ends, the join of its corners, a taper and whether its pattern is in
//! model distances everywhere. A section style (`ON_SectionStyle`) says how the cut of an object
//! by a clipping plane is drawn: its background fill, its boundary and its hatch. Rhino 8 keeps
//! section styles on layers and objects; Rhino 9 also has a section style table they refer to
//! by index. Both are written as items: a byte naming the item, its value, 0 at the end.

use super::{err, unit_scale, Cur, Uuid, R, TCODE_ANONYMOUS_CHUNK, TCODE_MODEL_ATTRIBUTES_CHUNK};
use threedm_model::{Attributes, Layer, LineCap, LineJoin, Linetype, SectionStyle, Stroke};

pub(super) const TCODE_LINETYPE_TABLE: u32 = 0x1000_0023;
pub(super) const TCODE_LINETYPE_RECORD: u32 = 0x2000_8078;
pub(super) const ON_LINETYPE: Uuid = Uuid::parse("26F10A24-7D13-4f05-8FDA-8E364DAF8EA6");
/// The section style table of Rhino 9 and its records (not in openNURBS 8: found in files
/// Rhino 9 writes, each record an `ON_SectionStyle` object as the other tables keep theirs).
pub(super) const TCODE_SECTION_STYLE_TABLE: u32 = 0x1000_0027;
pub(super) const TCODE_SECTION_STYLE_RECORD: u32 = 0x2000_807C;
pub(super) const ON_SECTION_STYLE: Uuid = Uuid::parse("6D0DE8B6-598B-4784-B2CA-A42BEB77B521");

/// Most points of a taper read.
const MAX_TAPER: usize = 10_000;

/// `ReadModelComponentAttributes`: each attribute after a byte saying it is there (1) or not;
/// the id, the index and the name.
pub(super) fn component(c: &mut Cur<'_>) -> R<(Uuid, Option<i32>, String)> {
    let (mut a, major, _) = c.versioned(TCODE_MODEL_ATTRIBUTES_CHUNK)?;
    if major != 1 {
        return err("model component attributes of an unknown version");
    }
    let (mut id, mut index, mut name) = (Uuid::default(), None, String::new());
    if a.u8()? == 1 {
        a.take(12)?; // serial numbers
    }
    if a.u8()? == 1 {
        id = a.uuid()?;
    }
    if a.u8()? == 1 {
        a.i32()?; // component type
    }
    if a.u8()? == 1 {
        index = Some(a.i32()?);
    }
    if a.u8()? == 1 {
        name = a.wstring()?;
    }
    Ok((id, index, name))
}

fn cap(v: u8) -> LineCap {
    match v {
        1 => LineCap::Flat,
        2 => LineCap::Square,
        _ => LineCap::Round,
    }
}

fn join(v: u8) -> LineJoin {
    match v {
        1 => LineJoin::Miter,
        2 => LineJoin::Bevel,
        _ => LineJoin::Round,
    }
}

/// `ON_Linetype::Read`: version 1 (before Rhino 6) its index, name, segments and id; version 2
/// its component attributes and segments, then from 2.1 the items cap (1) and join (2), from 2.2
/// width (3), width unit (4) and taper (5), from 2.3 always model distances (6). `position` is
/// its place in the table, for files that store no index.
pub(super) fn read_linetype(mut c: Cur<'_>, position: i32) -> R<Linetype> {
    let (mut b, major, minor) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    let mut lt = Linetype { index: position, ..Linetype::default() };
    match major {
        1 => {
            lt.index = b.i32()?;
            lt.name = b.wstring()?;
        }
        2 => {
            let (id, index, name) = component(&mut b)?;
            lt.id = id;
            lt.index = index.unwrap_or(position);
            lt.name = name;
        }
        _ => return err("a linetype of an unknown version"),
    }
    let n = b.count(12)?;
    for _ in 0..n {
        let length = b.f64()?;
        let kind = b.u32()?;
        let length = if length.is_finite() { length.abs() } else { 0.0 };
        lt.segments.push(if kind == 1 { -length } else { length });
    }
    if major == 1 {
        if minor >= 1 {
            lt.id = b.uuid()?;
        }
        return Ok(lt);
    }
    if minor < 1 {
        return Ok(lt);
    }
    // the items, in increasing order; one this reader does not know ends them
    let mut item = b.u8()?;
    if item == 1 {
        lt.cap = cap(b.u8()?);
        item = b.u8()?;
    }
    if item == 2 {
        lt.join = join(b.u8()?);
        item = b.u8()?;
    }
    if minor >= 2 {
        if item == 3 {
            lt.width = b.f64()?;
            item = b.u8()?;
        }
        if item == 4 {
            lt.width_unit = b.u8()?;
            item = b.u8()?;
        }
        if item == 5 {
            let n = b.count(16)?;
            for _ in 0..n {
                let p = [b.f64()?, b.f64()?];
                if lt.taper.len() < MAX_TAPER {
                    lt.taper.push(p);
                }
            }
            item = b.u8()?;
        }
    }
    if minor >= 3 && item == 6 {
        lt.always_model_distances = b.bool()?;
    }
    Ok(lt)
}

/// `ON_SectionStyle::Read` (version 1): its component attributes, then items: background fill
/// mode (1), fill colour and print colour (2), boundary visible (3), boundary colour and print
/// colour (4), boundary width scale (5), fill rule (6), hatch index (7), scale (8), rotation (9),
/// hatch colour and print colour (10), from 1.1 the boundary's linetype (11); Rhino 9 (1.3)
/// adds the boundary's print width (12), its linetype by index (13) and the hatch's print width
/// (14). `position` is its place in the table.
pub(super) fn read_section_style(c: &mut Cur<'_>, position: i32) -> R<SectionStyle> {
    let (mut b, major, _) = c.versioned(TCODE_ANONYMOUS_CHUNK)?;
    if major != 1 {
        return err("a section style of an unknown version");
    }
    let (id, index, name) = component(&mut b)?;
    let mut s = SectionStyle { id, index: index.unwrap_or(position), name, ..SectionStyle::default() };
    loop {
        match b.u8()? {
            1 => s.background_fill_mode = b.u8()?.min(2),
            2 => {
                s.background_fill_color = b.color()?;
                s.background_fill_print_color = b.color()?;
            }
            3 => s.boundary_visible = b.bool()?,
            4 => {
                s.boundary_color = b.color()?;
                s.boundary_print_color = b.color()?;
            }
            5 => s.boundary_width_scale = b.f64()?,
            6 => s.fill_rule = b.u8()?.min(1),
            7 => s.hatch_index = b.i32()?,
            8 => s.hatch_scale = b.f64()?,
            9 => s.hatch_rotation = b.f64()?,
            10 => {
                s.hatch_color = b.color()?;
                s.hatch_print_color = b.color()?;
            }
            11 => {
                // read from a copy, then passed by
                s.boundary_linetype = read_linetype(b.clone(), -1).ok();
                b.chunk()?;
            }
            12 => s.boundary_print_width = b.f64()?,
            13 => s.boundary_linetype_index = b.i32()?,
            14 => s.hatch_print_width = b.f64()?,
            // 0 ends them, and one this reader does not know
            _ => break,
        }
    }
    Ok(s)
}

/// How a curve's line is drawn: the linetype by object or by layer (or the object's own), its
/// width in model units (`meters` per model unit), and the print width and colour by object or
/// by layer.
pub(super) fn stroke(a: &Attributes, layers: &[Layer], linetypes: &[Linetype], meters: f64) -> Stroke {
    let layer = layers.iter().find(|l| l.index == a.layer);
    let index = match a.linetype_source {
        1 => a.linetype,
        _ => layer.map_or(-1, |l| l.linetype),
    };
    let continuous = Linetype::default();
    let (index, lt) = match &a.custom_linetype_style {
        Some(own) => (-1, own),
        None => match linetypes.iter().find(|l| index >= 0 && l.index == index) {
            Some(lt) => (index, lt),
            None => (-1, &continuous),
        },
    };
    let model_width = match lt.width_unit {
        0 => None,
        u => unit_scale(u as u32, None).filter(|_| meters > 0.0).map(|(m, _)| lt.width * m / meters),
    };
    let print_width = match a.plot_weight_source {
        1 => a.plot_weight,
        _ => layer.map_or(0.0, |l| l.plot_weight),
    };
    let display = match a.color_source {
        1 => a.color,
        _ => layer.map_or(a.color, |l| l.color),
    };
    let print_color = match a.plot_color_source {
        1 => a.plot_color,
        2 => display,
        _ => layer.map_or(a.plot_color, |l| l.plot_color),
    };
    Stroke { linetype: index, width: lt.width, width_unit: lt.width_unit, model_width, cap: lt.cap, join: lt.join, taper: lt.taper.clone(), print_width, print_color }
}
