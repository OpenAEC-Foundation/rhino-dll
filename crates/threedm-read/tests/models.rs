//! The test models (`test-models`, made in Rhino 9 by the scripts beside them) read as Rhino
//! reads them: every object, and the meshes made of the exact geometry as large as Rhino
//! measures it.

use std::collections::HashMap;
use glam::{DMat4, DVec3};
use threedm_read::{read, File, Geometry};

fn models() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-models")
}

fn load(name: &str) -> Vec<u8> {
    std::fs::read(models().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The area of the meshes of an object, in model units: of its members for a block instance,
/// placed by its transform.
fn area(file: &File<'_>, geometry: &Geometry<'_>, xform: DMat4, depth: usize) -> f64 {
    match geometry {
        Geometry::Meshes(meshes, _) => {
            let mut area = 0.0;
            for m in meshes {
                let p = |i: u32| xform.transform_point3(DVec3::from(m.positions[i as usize].map(f64::from)));
                let tri = |a: u32, b: u32, c: u32| 0.5 * (p(b) - p(a)).cross(p(c) - p(a)).length();
                for f in &m.faces {
                    area += tri(f[0], f[1], f[2]);
                    if f[2] != f[3] {
                        area += tri(f[0], f[2], f[3]);
                    }
                }
            }
            area
        }
        Geometry::Instance { definition, xform: placed } if depth < 8 => {
            let Some(block) = file.blocks.iter().find(|b| b.id == *definition) else { return 0.0 };
            let members = file.objects.iter().filter(|o| block.objects.contains(&o.attributes.id));
            members.map(|o| area(file, &o.geometry, xform * *placed, depth + 1)).sum()
        }
        _ => 0.0,
    }
}

/// The area of each named object in model space, in model units.
fn areas(file: &File<'_>) -> HashMap<String, f64> {
    let mut out = HashMap::new();
    // model space objects, not block definition geometry
    for o in file.objects.iter().filter(|o| o.attributes.mode != 3 && o.attributes.space != 2) {
        *out.entry(o.attributes.name.clone()).or_default() += area(file, &o.geometry, DMat4::IDENTITY, 0);
    }
    out
}

#[test]
fn every_model_reads_whole() {
    for path in std::fs::read_dir(models()).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "3dm")) {
        let data = std::fs::read(&path).unwrap();
        let file = read(&data).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(file.damaged.is_empty(), "{}: {:?}", path.display(), file.damaged);
        assert!(!file.objects.is_empty(), "{}", path.display());
        assert_eq!(file.versions.archive, 90, "{}", path.display());
    }
}

#[test]
fn meshed_areas_match_rhino() {
    // file, object name, area in model units (square millimetres) as Rhino 9 computes it
        let expected = [
        ("primitives", "Sphere", 314.1593),
        ("primitives", "Cylinder", 351.8584),
        ("primitives", "Cone", 185.6094),
        ("primitives", "Torus", 394.7842),
        ("primitives", "Box", 448.0000),
        ("primitives", "Plane", 560.0000),
        ("primitives", "Wavy patch", 329.3163),
        ("booleans", "Plate with holes", 999.0885),
        ("booleans", "Box and tower", 510.7964),
        ("booleans", "Sphere with a tunnel", 403.0810),
        ("extrusions", "Column", 89.5354),
        ("extrusions", "Tube", 257.9200),
        ("extrusions", "I-beam", 125.0400),
        ("extrusions", "Wave wall", 34.4615),
        ("extrusions", "Beam", 81.2000),
        ("freeform", "Vase", 139.9704),
        ("freeform", "Revolved", 120.2451),
        ("freeform", "Sweep", 59.8317),
        ("freeform", "Patch with a hole", 127.7938),
        ("freeform", "Edge surface", 44.4351),
        ("building", "Window 1", 5.0480),
        ("building", "Window 2", 5.0480),
        ("building", "Window 3", 5.0480),
        ("building", "Window 4", 5.0480),
        ("building", "Walls", 317.0467),
        ("building", "Ground slab", 390.8000),
        ("building", "Roof", 440.9390),
        ("building", "Column 1", 4.0212),
        ("building", "Column 2", 4.0212),
        ("building", "Column 3", 4.0212),
        ("building", "Column 4", 4.0212),
        ("building", "Column 5", 4.0212),
        ("tower", "Twisted tower", 2152.8922),
        ("tower", "Balcony 1", 299.8000),
        ("tower", "Balcony 2", 299.8000),
        ("tower", "Balcony 3", 299.8000),
        ("tower", "Balcony 4", 299.8000),
        ("tower", "Balcony 5", 299.8000),
        ("tower", "Balcony 6", 299.8000),
        ("tower", "Balcony 7", 299.8000),
        ("tower", "Ring slab", 202.9469),
        ("tower", "Canopy", 175.5441),
        ("mechanical", "Flange", 855.8850),
        ("mechanical", "Gear", 316.6037),
        ("mechanical", "Bracket", 220.2145),
        ("stairs", "Column", 11.8752),
        ("stairs", "Step 1", 3.5360),
        ("stairs", "Step 2", 3.5360),
        ("stairs", "Step 3", 3.5360),
        ("stairs", "Step 4", 3.5360),
        ("stairs", "Step 5", 3.5360),
        ("stairs", "Step 6", 3.5360),
        ("stairs", "Step 7", 3.5360),
        ("stairs", "Step 8", 3.5360),
        ("stairs", "Step 9", 3.5360),
        ("stairs", "Step 10", 3.5360),
        ("stairs", "Step 11", 3.5360),
        ("stairs", "Step 12", 3.5360),
        ("stairs", "Step 13", 3.5360),
        ("stairs", "Step 14", 3.5360),
        ("stairs", "Step 15", 3.5360),
        ("stairs", "Step 16", 3.5360),
        ("stairs", "Step 17", 3.5360),
        ("stairs", "Step 18", 3.5360),
        ("stairs", "Step 19", 3.5360),
        ("stairs", "Step 20", 3.5360),
        ("stairs", "Handrail", 3.6224),
        ("dome", "Dome", 408.1008),
        ("dome", "Drum", 320.6938),
        ("roofs", "Hypar", 125.2610),
        ("roofs", "SubD blob", 106.3228),
        ("roofs", "Ribbon", 30.7790),
        // three table sets of four chairs (nested blocks) and a mirrored chair
        ("blocks", "", 26.1531),
        ];
    let mut files = HashMap::new();
    let mut wrong = Vec::new();
    for (name, object, rhino) in expected {
        let data = files.entry(name).or_insert_with(|| load(&format!("{name}.3dm")));
        let file = read(data).unwrap();
        let areas = areas(&file);
        // an empty name: the whole file
        let ours: f64 = areas.iter().filter(|(o, _)| object.is_empty() || o.as_str() == object).map(|(_, a)| a).sum();
        if (ours - rhino).abs() > 0.015 * rhino {
            wrong.push(format!("{name} {object}: {ours:.3} against {rhino} in Rhino"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Curves, texts and hatches (`drafting.3dm`). Its dimension style draws texts twice their height
/// in model space; each text covers the area Rhino's outlines of it cover (in Inter, so a little
/// narrower or wider than Rhino's Arial).
#[test]
fn texts_are_as_large_as_in_rhino() {
    let data = load("drafting.3dm");
    let file = read(&data).unwrap();
    let kinds = |kind: &str| file.objects.iter().filter(|o| match &o.geometry {
        Geometry::Meshes(_, k) | Geometry::Lines(_, k) => *k == kind,
        _ => false,
    }).count();
    assert_eq!((kinds("Curve"), kinds("Text"), kinds("Hatch")), (7, 6, 5));
    // a grid is lines both ways, a plus pattern dashes both ways (the second line family's
    // offset is in its own frame)
    for name in ["Grid", "Plus"] {
        let object = file.objects.iter().find(|o| o.attributes.name == name).unwrap();
        let Geometry::Lines(lines, _) = &object.geometry else { panic!("{name} is not drawn in lines") };
        let along = |horizontal: bool| lines.iter().filter(|l| {
            let d = l[l.len() - 1] - l[0];
            if horizontal { d.y.abs() < 1e-9 && d.x.abs() > 1e-9 } else { d.x.abs() < 1e-9 && d.y.abs() > 1e-9 }
        }).count();
        assert!(along(true) > 3 && along(false) > 3, "{name}: {} across, {} up", along(true), along(false));
    }
    // Rhino's outlines of the texts: x and y from and to
    let rhino = [
        ("Text 1", [0.1097, -12.0170, 8.1377, -10.9966]),
        ("Text 2", [0.1858, -16.0426, 8.7031, -13.4591]),
        ("Text 3", [15.0327, -12.2938, 18.7485, -9.4007]),
        ("Text 4", [24.7322, -11.8019, 31.4700, -7.4428]),
        ("Text 5", [37.8264, -12.5164, 42.2054, -11.5007]),
        ("Text 6", [0.0545, -20.1977, 8.1241, -18.9836]),
    ];
    for (name, [x0, y0, x1, y1]) in rhino {
        let object = file.objects.iter().find(|o| o.attributes.name == name).unwrap();
        let Geometry::Meshes(meshes, _) = &object.geometry else { panic!("{name} is not drawn") };
        let (mut low, mut high) = (DVec3::MAX, DVec3::MIN);
        for p in meshes.iter().flat_map(|m| &m.positions) {
            let p = DVec3::from(p.map(f64::from));
            low = low.min(p);
            high = high.max(p);
        }
        let (w, h) = (x1 - x0, y1 - y0);
        let size = high - low;
        assert!((size.y - h).abs() < 0.15 * h, "{name}: {} high against {h} in Rhino", size.y);
        assert!((size.x - w).abs() < 0.2 * w, "{name}: {} wide against {w} in Rhino", size.x);
        let mid = (low + high) / 2.0;
        assert!((mid.x - (x0 + x1) / 2.0).abs() < 0.15 * w && (mid.y - (y0 + y1) / 2.0).abs() < 0.15 * h, "{name}: around {mid}");
    }
}

/// Dimensions and leaders (`dimensions.3dm`, from make_dimensions.py) as Rhino 9 draws them:
/// `dimensions.json` holds what Rhino makes of each when exploded. Every line of Rhino's
/// (arrowheads aside) has one of ours with the same ends, and each text covers Rhino's text box
/// (in Inter, so a little narrower or wider than Rhino's Arial).
#[test]
fn dimensions_and_leaders_are_drawn_as_in_rhino() {
    let data = load("dimensions.3dm");
    let file = read(&data).unwrap();
    let reference: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(models().join("dimensions.json")).unwrap()).unwrap();
    let near = |p: DVec3, q: &serde_json::Value, d: f64| (p.x - q[0].as_f64().unwrap()).abs() < d && (p.y - q[1].as_f64().unwrap()).abs() < d;
    let mut wrong = Vec::new();
    for (name, rhino) in reference.as_object().unwrap() {
        let Some(object) = file.objects.iter().find(|o| &o.attributes.name == name) else {
            wrong.push(format!("{name}: not read"));
            continue;
        };
        let Geometry::Drawing { lines, meshes, .. } = &object.geometry else {
            wrong.push(format!("{name}: not drawn ({:?})", std::mem::discriminant(&object.geometry)));
            continue;
        };
        // an ordinate's line runs on as far as its text, which is set in another font
        let d = if name.starts_with("Ordinate") { 0.3 } else { 0.02 };
        let close = |p: DVec3, q: &serde_json::Value| near(p, q, d);
        for line in rhino["lines"].as_array().unwrap().iter().filter_map(|l| l.as_array()) {
            let (a, b) = (&line[0], &line[line.len() - 1]);
            if a == b {
                continue; // an arrowhead
            }
            let found = lines.iter().any(|l| {
                let (p, q) = (l[0], l[l.len() - 1]);
                (close(p, a) && close(q, b)) || (close(p, b) && close(q, a))
            });
            if !found {
                wrong.push(format!("{name}: no line from {a} to {b}"));
            }
        }
        // the text: the meshes of more than a few points (arrowheads have at most 16)
        let (mut low, mut high) = (DVec3::MAX, DVec3::MIN);
        for p in meshes.iter().filter(|m| m.positions.len() > 24).flat_map(|m| &m.positions) {
            let p = DVec3::from(p.map(f64::from));
            low = low.min(p);
            high = high.max(p);
        }
        for t in rhino["texts"].as_array().unwrap() {
            let b: Vec<f64> = t["box"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
            let (w, h) = (b[2] - b[0], b[3] - b[1]);
            let (mid_x, mid_y) = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
            let size = high - low;
            let mid = (low + high) / 2.0;
            // a text turned a quarter round is as wide as it is long
            let (ours_w, ours_h) = (size.x, size.y);
            let ok = (mid.x - mid_x).abs() < 0.3 && (mid.y - mid_y).abs() < 0.3 && (ours_w - w).abs() < 0.25 * w.max(h) && (ours_h - h).abs() < 0.25 * w.max(h);
            if !ok {
                wrong.push(format!("{name}: text {} at {mid_x:.2}, {mid_y:.2}, {w:.2} by {h:.2}; ours at {:.2}, {:.2}, {:.2} by {:.2}", t["text"], mid.x, mid.y, ours_w, ours_h));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
