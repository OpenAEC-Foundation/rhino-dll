//! Lights: the records of the light table (`Read3dmLight`), an `ON_Light` and its object
//! attributes, and lights kept in the object table.

use glam::DVec3;
use threedm_core::*;
use threedm_model::{Light, LightKind};

pub(crate) const TCODE_LIGHT_TABLE: u32 = 0x1000_0012;
pub(crate) const TCODE_LIGHT_RECORD: u32 = 0x2000_8060;
const TCODE_LIGHT_RECORD_ATTRIBUTES: u32 = 0x0200_8061;
const TCODE_LIGHT_RECORD_END: u32 = 0x8200_006F;
const ON_LIGHT: Uuid = Uuid::parse("85A08513-F383-11d3-BFE7-0010830122F0");
/// The object type of a light in the object table (`ON::light_object`).
const LIGHT_OBJECT: u32 = 0x100;
/// At most this many lights are read.
pub(crate) const MAX_LIGHTS: usize = 10_000;

/// A record of the light table: the light, then its attributes, up to the record's end.
pub(crate) fn read_light_record(mut c: Cur<'_>) -> R<Option<Light>> {
    let Some(object) = c.object()?.filter(|o| o.class == ON_LIGHT) else { return Ok(None) };
    let mut light = read_light(object.data)?;
    while c.left() > 0 {
        let chunk = c.chunk()?;
        match chunk.typecode {
            TCODE_LIGHT_RECORD_ATTRIBUTES => attributes(&mut light, chunk.body)?,
            TCODE_LIGHT_RECORD_END => break,
            _ => {}
        }
    }
    Ok(Some(light))
}

/// An object record of the object table that holds a light (object type `0x100`): the light
/// with its attributes; `None` for any other object.
pub(crate) fn read_object_light(mut c: Cur<'_>) -> R<Option<Light>> {
    let kind = c.chunk()?;
    if kind.typecode != TCODE_OBJECT_RECORD_TYPE || kind.value as u32 != LIGHT_OBJECT {
        return Ok(None);
    }
    let Some(object) = c.object()?.filter(|o| o.class == ON_LIGHT) else { return Ok(None) };
    let mut light = read_light(object.data)?;
    while c.left() > 0 {
        let chunk = c.chunk()?;
        match chunk.typecode {
            TCODE_OBJECT_RECORD_ATTRIBUTES => attributes(&mut light, chunk.body)?,
            TCODE_OBJECT_RECORD_END => break,
            _ => {}
        }
    }
    Ok(Some(light))
}

/// The layer and visibility of a light's attributes, and its id and name when it has none.
fn attributes(light: &mut Light, c: Cur<'_>) -> R<()> {
    let a = super::read_attributes(c)?;
    light.layer = a.layer;
    light.visible = a.visible;
    if light.id.is_nil() {
        light.id = a.id;
    }
    if light.name.is_empty() {
        light.name = a.name;
    }
    Ok(())
}

/// The spot exponent and hot spot as `ON_Light::SpotExponent` and `HotSpot` give them: a light
/// keeps one of the two (a hot spot from 0 to 1, or else an exponent), the other follows from
/// cos(hot spot · angle)^exponent = √½.
fn spot(angle: f64, exponent: f64, hotspot: f64) -> (f64, f64) {
    let log_min = std::f64::consts::FRAC_1_SQRT_2.ln();
    let usable = angle > 0.0 && angle <= 90.0;
    if (0.0..=1.0).contains(&hotspot) {
        let h = hotspot.max(0.015);
        let e = if h >= 1.0 || !usable {
            0.0
        } else {
            let c = (h * angle.to_radians()).cos();
            if c <= 0.0 { 1.0 } else { (log_min / c.ln()).max(0.0) }
        };
        return (e, hotspot);
    }
    let h = if exponent >= 65536.0 {
        0.0
    } else if exponent <= 0.0 || !usable {
        1.0
    } else {
        let x = log_min / exponent;
        if x < -690.0 {
            1.0
        } else {
            let c = x.exp();
            let c = if c.is_finite() { c.clamp(-1.0, 1.0) } else { 0.0 };
            (c.acos() / angle.to_radians()).clamp(0.0, 1.0)
        }
    };
    (exponent, h)
}

/// `ON_Light::Read` (version 1): on, style, intensity, watts, the ambient, diffuse and specular
/// colours, direction, location, spot angle and exponent, attenuation, shadow intensity, index,
/// id and name; the length and width (1.1) and the hot spot (1.2; before, from the exponent).
fn read_light(mut c: Cur<'_>) -> R<Light> {
    let (major, minor) = c.version()?;
    if major != 1 {
        return err("a light of an unknown version");
    }
    let vector = |c: &mut Cur<'_>| -> R<DVec3> { Ok(DVec3::new(c.f64()?, c.f64()?, c.f64()?)) };
    let on = c.i32()? != 0;
    let style = c.i32()?;
    let intensity = c.f64()?;
    let watts = c.f64()?;
    let ambient = c.color()?;
    let diffuse = c.color()?;
    let specular = c.color()?;
    let direction = vector(&mut c)?;
    let location = vector(&mut c)?;
    let spot_angle = c.f64()?;
    let mut spot_exponent = c.f64()?;
    let attenuation = vector(&mut c)?;
    let shadow_intensity = c.f64()?;
    c.i32()?; // index
    let id = c.uuid()?;
    let name = c.wstring()?;
    let mut hotspot = 1.0;
    if minor < 2 {
        hotspot = (1.0 - spot_exponent / 128.0).clamp(0.0, 1.0);
        spot_exponent = 0.0;
    }
    let (mut length, mut width) = (DVec3::ZERO, DVec3::ZERO);
    if minor >= 1 {
        length = vector(&mut c)?;
        width = vector(&mut c)?;
        if minor >= 2 {
            hotspot = c.f64()?;
        }
    }
    let (spot_exponent, hotspot) = spot(spot_angle, spot_exponent, hotspot);
    let (kind, camera) = match style {
        4 => (LightKind::Directional, true),
        5 => (LightKind::Point, true),
        6 => (LightKind::Spot, true),
        7 => (LightKind::Directional, false),
        8 => (LightKind::Point, false),
        9 => (LightKind::Spot, false),
        10 => (LightKind::Ambient, false),
        11 => (LightKind::Linear, false),
        12 => (LightKind::Rectangular, false),
        _ => (LightKind::Unknown, false),
    };
    Ok(Light {
        id,
        name,
        layer: 0,
        visible: true,
        on,
        kind,
        camera,
        style,
        location,
        direction,
        length,
        width,
        ambient,
        diffuse,
        specular,
        intensity,
        watts,
        spot_angle,
        spot_exponent,
        hotspot,
        attenuation,
        shadow_intensity,
    })
}
