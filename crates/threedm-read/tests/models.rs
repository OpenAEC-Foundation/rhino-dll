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
        // Rhino 9 files, and one saved by it as Rhino 5
        let rhino5 = path.file_stem().is_some_and(|s| s.to_string_lossy().ends_with("_v5"));
        assert_eq!(file.versions.archive, if rhino5 { 50 } else { 90 }, "{}", path.display());
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
        // no dash cut to nothing by the boundary
        let shortest = lines.iter().map(|l| l[0].distance(l[l.len() - 1])).fold(f64::INFINITY, f64::min);
        assert!(shortest > 1e-6, "{name}: a piece {shortest} long");
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

/// Curves in Rhino's linetypes (`linetypes.3dm`, from make_linetypes.py, in millimetres): a line
/// 100 long in Dashed (5 dash, 5 gap) is half a dash, nine dashes and half a dash, 50 drawn in
/// all, as Rhino draws it; Continuous stays whole; a circle takes the linetype of its layer.
#[test]
fn curves_take_their_linetype() {
    let data = load("linetypes.3dm");
    let file = read(&data).unwrap();
    let lines = |name: &str| match &file.objects.iter().find(|o| o.attributes.name == name).unwrap().geometry {
        Geometry::Lines(l, _) => l.clone(),
        other => panic!("{name}: {:?}", std::mem::discriminant(other)),
    };
    let drawn = |l: &[Vec<DVec3>]| l.iter().map(|p| p.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>()).sum::<f64>();
    assert_eq!(lines("Continuous").len(), 1);
    let dashed = lines("Dashed");
    assert_eq!(dashed.len(), 11, "half, nine and half a dash");
    assert!((drawn(&dashed) - 50.0).abs() < 1e-6, "{}", drawn(&dashed));
    assert!((dashed[0][0].x - 0.0).abs() < 1e-9 && (dashed[0][1].x - 2.5).abs() < 1e-9);
    // Hidden: 2 dash, 2 gap; Dots: dots one apart
    assert!((drawn(&lines("Hidden")) - 50.0).abs() < 1e-6);
    assert!(lines("Dots").len() >= 99);
    // by layer: the circle, 30 across, dashed along its length
    let circle = lines("Circle by layer");
    assert!(circle.len() > 8, "{} pieces", circle.len());
    let around = std::f64::consts::PI * 30.0;
    assert!((drawn(&circle) - around / 2.0).abs() < 0.05 * around, "{} of {around}", drawn(&circle));
}

/// The dimensions and leaders of `dimensions.3dm` saved by Rhino 9 as a Rhino 5 file
/// (`dimensions_v5.3dm`: Rhino 4 and 5 annotations and dimension styles), against what Rhino 9
/// draws of that file (`dimensions_v5.json`, ids and 3d points): every line of Rhino's (arrowheads
/// aside) is one of ours, and the texts of the dimensions sit where Rhino's do.
#[test]
fn rhino5_dimensions_are_drawn_as_in_rhino() {
    let data = load("dimensions_v5.3dm");
    let file = read(&data).unwrap();
    let reference: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(models().join("dimensions_v5.json")).unwrap()).unwrap();
    let close = |p: DVec3, q: &serde_json::Value| (0..3).all(|k| (p[k] - q[k].as_f64().unwrap()).abs() < 0.05);
    let mut wrong = Vec::new();
    for (id, rhino) in reference.as_object().unwrap() {
        let Some(object) = file.objects.iter().find(|o| o.attributes.id.to_string().eq_ignore_ascii_case(id)) else {
            wrong.push(format!("{id}: not read"));
            continue;
        };
        let Geometry::Drawing { lines, meshes, kind } = &object.geometry else {
            wrong.push(format!("{id}: not drawn"));
            continue;
        };
        for line in rhino["lines"].as_array().unwrap() {
            let (a, b) = (&line[0], &line[1]);
            if a == b {
                continue;
            }
            if !lines.iter().any(|l| (close(l[0], a) && close(l[l.len() - 1], b)) || (close(l[0], b) && close(l[l.len() - 1], a))) {
                wrong.push(format!("{id} {kind}: no line from {a} to {b}"));
            }
        }
        // the texts of the dimensions (Rhino turns the text of a straight Rhino 5 leader along it)
        if *kind != "Dimension" {
            continue;
        }
        let (mut low, mut high) = (DVec3::MAX, DVec3::MIN);
        for p in meshes.iter().filter(|m| m.positions.len() > 24).flat_map(|m| &m.positions) {
            let p = DVec3::from(p.map(f64::from));
            low = low.min(p);
            high = high.max(p);
        }
        for t in rhino["texts"].as_array().unwrap() {
            let b: Vec<f64> = t["box"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
            let mid = DVec3::new((b[0] + b[3]) / 2.0, (b[1] + b[4]) / 2.0, (b[2] + b[5]) / 2.0);
            let ours = (low + high) / 2.0;
            if ours.distance(mid) > 0.8 {
                wrong.push(format!("{id}: text {} at {mid}, ours at {ours}", t["text"]));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Clipping planes (`clipping.3dm`, from make_clipping.py) as Rhino 9 reads them
/// (`clipping.json`): their rectangle, plane, the viewports they clip and their depth; and the
/// file's views, the Perspective viewport first.
#[test]
fn clipping_planes_are_read_as_in_rhino() {
    let data = load("clipping.3dm");
    let file = read(&data).unwrap();
    let reference: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(models().join("clipping.json")).unwrap()).unwrap();
    let views: Vec<_> = file.views.iter().map(|v| (v.id.to_string(), v.name.clone(), v.perspective)).collect();
    let rhino: Vec<_> = reference["views"].as_array().unwrap().iter().map(|v| (v["id"].as_str().unwrap().to_string(), v["name"].as_str().unwrap().to_string(), v["perspective"].as_bool().unwrap())).collect();
    assert_eq!(views, rhino);
    let point = |q: &serde_json::Value| DVec3::new(q[0].as_f64().unwrap(), q[1].as_f64().unwrap(), q[2].as_f64().unwrap());
    let planes = reference["planes"].as_array().unwrap();
    assert_eq!(planes.len(), 3);
    for r in planes {
        let name = r["name"].as_str().unwrap();
        let o = file.objects.iter().find(|o| o.attributes.name == name).unwrap_or_else(|| panic!("{name}"));
        let Geometry::ClippingPlane(p) = &o.geometry else { panic!("{name}: not a clipping plane") };
        assert!(p.origin.distance(point(&r["origin"])) < 1e-9, "{name}");
        assert!(p.normal.distance(point(&r["normal"])) < 1e-9, "{name}");
        for (c, q) in p.corners.iter().zip(r["corners"].as_array().unwrap()) {
            assert!(c.distance(point(q)) < 1e-9, "{name}: {c} {q}");
        }
        let viewports: Vec<String> = p.viewports.iter().map(|v| v.to_string()).collect();
        let rhino: Vec<String> = r["viewports"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
        assert_eq!(viewports, rhino, "{name}");
        assert_eq!(p.depth, r["depth"].as_f64(), "{name}");
        assert!(p.enabled && p.selection.is_none(), "{name}");
    }
}

/// Named views (`views.3dm`, from make_views.py) as Rhino 9 reads them (`views.json`): their
/// names, projection, camera and field of view.
#[test]
fn named_views_are_read_as_in_rhino() {
    let data = load("views.3dm");
    let file = read(&data).unwrap();
    let reference: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(models().join("views.json")).unwrap()).unwrap();
    let point = |q: &serde_json::Value| DVec3::new(q[0].as_f64().unwrap(), q[1].as_f64().unwrap(), q[2].as_f64().unwrap());
    let rhino = reference.as_array().unwrap();
    assert_eq!(file.named_views.len(), rhino.len());
    for (v, r) in file.named_views.iter().zip(rhino) {
        let name = r["name"].as_str().unwrap();
        assert_eq!((v.name.as_str(), v.perspective), (name, r["perspective"].as_bool().unwrap()));
        assert!(v.location.distance(point(&r["location"])) < 1e-9, "{name}");
        assert!(v.direction.distance(point(&r["direction"]).normalize()) < 1e-9, "{name}");
        assert!(v.up.distance(point(&r["up"]).normalize()) < 1e-9, "{name}");
        let f: Vec<f64> = r["frustum"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
        assert!((v.height - (f[3] - f[2])).abs() < 1e-9, "{name}");
        if v.perspective {
            assert!((v.fov_y - 2.0 * (0.5 * (f[3] - f[2]) / f[4]).atan()).abs() < 1e-12, "{name}");
        }
    }
}

/// Object attributes (`attributes.3dm`, from make_attributes.py) as Rhino 9 reads them
/// (`attributes.json`): groups (and the group table), url, print colour and width, linetype
/// scale, an object's own linetype, draw order, a hatch's background fill and boundary. A line
/// 100 long in Dashed at scale 2 draws half of it; one in its own 4-1 linetype four fifths.
#[test]
fn object_attributes_are_read_as_in_rhino() {
    let data = load("attributes.3dm");
    let file = read(&data).unwrap();
    let reference: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(models().join("attributes.json")).unwrap()).unwrap();
    let groups: Vec<(i32, String)> = reference["groups"].as_array().unwrap().iter().map(|g| (g["index"].as_i64().unwrap() as i32, g["name"].as_str().unwrap().to_string())).collect();
    assert_eq!(file.groups, groups);
    for r in reference["objects"].as_array().unwrap() {
        let name = r["name"].as_str().unwrap();
        let a = &file.objects.iter().find(|o| o.attributes.name == name).unwrap_or_else(|| panic!("{name}")).attributes;
        let ints = |v: &serde_json::Value| v.as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect::<Vec<_>>();
        assert_eq!(a.groups.iter().map(|&g| g as i64).collect::<Vec<_>>(), ints(&r["groups"]), "{name}");
        assert_eq!(a.url, r["url"].as_str().unwrap_or(""), "{name}");
        assert_eq!(a.plot_color[..3].iter().map(|&c| c as i64).collect::<Vec<_>>(), ints(&r["plot_color"]), "{name}");
        assert!((a.plot_weight - r["plot_weight"].as_f64().unwrap()).abs() < 1e-12, "{name}");
        assert_eq!((a.plot_color_source as i64, a.plot_weight_source as i64), (r["plot_color_source"].as_i64().unwrap(), r["plot_weight_source"].as_i64().unwrap()), "{name}");
        assert_eq!(a.linetype_scale, r["linetype_scale"].as_f64().unwrap(), "{name}");
        assert_eq!(a.display_order as i64, r["display_order"].as_i64().unwrap(), "{name}");
        assert_eq!(a.custom_linetype, r["custom_linetype"].as_array().map(|v| v.iter().map(|x| x.as_f64().unwrap()).collect::<Vec<_>>()), "{name}");
        assert_eq!(a.hatch_background.map(|c| c.iter().map(|&x| x as i64).collect::<Vec<_>>()), r["hatch_background"].as_array().map(|_| ints(&r["hatch_background"])), "{name}");
        assert_eq!(a.hatch_boundary, r["hatch_boundary"].as_bool().unwrap(), "{name}");
    }
    let drawn = |name: &str| match &file.objects.iter().find(|o| o.attributes.name == name).unwrap().geometry {
        Geometry::Lines(l, _) => l.iter().map(|p| p.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>()).sum::<f64>(),
        _ => panic!("{name}"),
    };
    assert!((drawn("Grouped") - 50.0).abs() < 1e-6, "{}", drawn("Grouped"));
    assert!((drawn("Own linetype") - 80.0).abs() < 1.0, "{}", drawn("Own linetype"));
}

/// Lights (`lights.3dm`, from make_lights.py) as Rhino 9 reads them (`lights.json`): one of each
/// kind, with every field of the light and the id, name, layer and visibility of its attributes.
#[test]
fn lights_are_read_as_in_rhino() {
    let data = load("lights.3dm");
    let file = read(&data).unwrap();
    let reference: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(models().join("lights.json")).unwrap()).unwrap();
    let point = |q: &serde_json::Value| DVec3::new(q[0].as_f64().unwrap(), q[1].as_f64().unwrap(), q[2].as_f64().unwrap());
    let color = |q: &serde_json::Value| -> [u8; 4] { std::array::from_fn(|i| q[i].as_u64().unwrap() as u8) };
    let number = |q: &serde_json::Value| q.as_f64().unwrap();
    let rhino = reference["lights"].as_array().unwrap();
    assert_eq!(file.lights.len(), rhino.len());
    // the lights are not objects: only the box is
    assert_eq!(file.objects.len(), 1);
    for r in rhino {
        let name = r["name"].as_str().unwrap();
        let l = file.lights.iter().find(|l| l.name == name).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(l.id.to_string(), r["id"].as_str().unwrap(), "{name}");
        assert_eq!(l.id.to_string(), r["attributes_id"].as_str().unwrap(), "{name}");
        assert_eq!((l.layer as i64, l.visible, l.on, l.style as i64), (r["layer"].as_i64().unwrap(), r["visible"].as_bool().unwrap(), r["enabled"].as_bool().unwrap(), r["style"].as_i64().unwrap()), "{name}");
        for (field, ours) in [("location", l.location), ("direction", l.direction), ("length", l.length), ("width", l.width), ("attenuation", l.attenuation)] {
            assert!(ours.distance(point(&r[field])) < 1e-12, "{name} {field}: {ours} {}", r[field]);
        }
        for (field, ours) in [("ambient", l.ambient), ("diffuse", l.diffuse), ("specular", l.specular)] {
            assert_eq!(ours, color(&r[field]), "{name} {field}");
        }
        for (field, ours) in [("intensity", l.intensity), ("watts", l.watts), ("spot_angle", l.spot_angle.to_radians()), ("spot_exponent", l.spot_exponent), ("hotspot", l.hotspot), ("shadow_intensity", l.shadow_intensity)] {
            assert!((ours - number(&r[field])).abs() < 1e-12, "{name} {field}: {ours} {}", r[field]);
        }
    }
    use threedm_read::LightKind::*;
    let kinds: Vec<_> = file.lights.iter().map(|l| (l.name.as_str(), l.kind, l.camera)).collect();
    for kind in [("Point", Point, false), ("Spot", Spot, false), ("Directional", Directional, false), ("Linear", Linear, false), ("Rectangular", Rectangular, false), ("Ambient", Ambient, false), ("Headlight", Directional, true)] {
        assert!(kinds.contains(&kind), "{kind:?}");
    }
}

/// The lights of McNeel's openNURBS example files (not in this repository: set
/// `OPENNURBS_EXAMPLES` to the `example_files` folder of an openNURBS checkout): all 27, in 14
/// files, read whole.
#[test]
fn lights_of_the_opennurbs_examples_are_read() {
    let Ok(folder) = std::env::var("OPENNURBS_EXAMPLES") else { return };
    let mut files = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(folder)];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("3dm")) {
                files.push(path);
            }
        }
    }
    let (mut lights, mut with_lights) = (0, 0);
    for path in &files {
        let data = std::fs::read(path).unwrap();
        let Ok(file) = read(&data) else { continue };
        assert!(!file.damaged.contains_key("lights"), "{}", path.display());
        if !file.lights.is_empty() {
            with_lights += 1;
            lights += file.lights.len();
        }
        for l in &file.lights {
            assert!(l.kind != threedm_read::LightKind::Unknown && l.location.is_finite() && l.direction.is_finite(), "{}: {l:?}", path.display());
        }
    }
    assert_eq!((lights, with_lights), (27, 14));
}

/// Materials (`materials.3dm`, from make_materials.py) as Rhino 9 reads them
/// (`materials.json`): their colours, shine, reflectivity, index of refraction, transparency,
/// textures and physically based parameters; and the texture coordinates of a mesh.
#[test]
fn materials_are_read_as_in_rhino() {
    let data = load("materials.3dm");
    let file = read(&data).unwrap();
    let reference: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(models().join("materials.json")).unwrap()).unwrap();
    let f = |v: &serde_json::Value| v.as_f64().unwrap();
    let floats = |v: &serde_json::Value| v.as_array().unwrap().iter().map(f).collect::<Vec<_>>();
    // a colour as RhinoCommon gives it: alpha 255 opaque
    let rgba = |c: [u8; 4]| vec![c[0] as f64, c[1] as f64, c[2] as f64, (255 - c[3]) as f64];
    let rhino = reference["materials"].as_array().unwrap();
    assert_eq!(file.materials.len(), rhino.len());
    for (m, r) in file.materials.iter().zip(rhino) {
        let name = r["name"].as_str().unwrap();
        assert_eq!((m.name.as_str(), m.index as i64), (name, r["index"].as_i64().unwrap()));
        for (color, key) in [(m.ambient, "ambient"), (m.diffuse, "diffuse"), (m.emission, "emission"), (m.specular, "specular"), (m.reflection, "reflection"), (m.transparent, "transparent")] {
            assert_eq!(rgba(color), floats(&r[key]), "{name} {key}");
        }
        for (value, key) in [(m.shine, "shine"), (m.reflectivity, "reflectivity"), (m.index_of_refraction, "ior"), (m.transparency, "transparency")] {
            assert!((value - f(&r[key])).abs() < 1e-12, "{name} {key}");
        }
        let textures = r["textures"].as_array().unwrap();
        assert_eq!(m.textures.len(), textures.len(), "{name}");
        for (t, q) in m.textures.iter().zip(textures) {
            assert_eq!(t.file, q["file"].as_str().unwrap(), "{name}");
            assert_eq!((t.kind as i64, t.mode as i64, t.enabled), (q["type"].as_i64().unwrap(), q["mode"].as_i64().unwrap(), q["enabled"].as_bool().unwrap()), "{name}");
            assert_eq!(t.amount, f(&q["blend"]), "{name}");
            // RhinoCommon's transform row by row
            let uvw: Vec<f64> = (0..16).map(|k| t.uvw.col(k % 4)[k / 4]).collect();
            assert_eq!(uvw, floats(&q["uvw"]), "{name}");
        }
        match (&m.pbr, &r["pbr"]) {
            (None, serde_json::Value::Null) => {}
            (Some(p), q) if q.is_object() => {
                assert_eq!(p.base_color.map(f64::from).to_vec(), floats(&q["base_color"]), "{name}");
                assert_eq!(p.emission.map(f64::from).to_vec(), floats(&q["emission"]), "{name}");
                for (value, key) in [(p.metallic, "metallic"), (p.roughness, "roughness"), (p.specular, "specular"), (p.opacity, "opacity"), (p.opacity_ior, "opacity_ior"), (p.clearcoat, "clearcoat"), (p.alpha, "alpha")] {
                    assert!((value - f(&q[key])).abs() < 1e-12, "{name} {key}");
                }
            }
            (p, q) => panic!("{name}: {p:?} {q}"),
        }
    }
    let meshes = reference["meshes"].as_array().unwrap();
    assert!(!meshes.is_empty());
    for r in meshes {
        let name = r["name"].as_str().unwrap();
        let o = file.objects.iter().find(|o| o.attributes.name == name).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(o.attributes.material as i64, r["material"].as_i64().unwrap(), "{name}");
        let Geometry::Meshes(m, _) = &o.geometry else { panic!("{name}: not a mesh") };
        let uv: Vec<Vec<f64>> = m[0].texture_coordinates.as_ref().unwrap().iter().map(|t| t.map(f64::from).to_vec()).collect();
        let rhino: Vec<Vec<f64>> = r["texture_coordinates"].as_array().unwrap().iter().map(floats).collect();
        assert_eq!(uv, rhino, "{name}");
    }
    assert_eq!(file.bitmaps.len(), reference["bitmaps"].as_array().unwrap().len());
}

/// Layouts (`layouts.3dm`, from make_layouts.py) as Rhino 9 reads them (`layouts.json`): the
/// pages among the views with their paper and page number; of each detail its rectangle on the
/// page, projection, camera, scale and lock; and the page each page object is on.
#[test]
fn layouts_and_details_are_read_as_in_rhino() {
    let data = load("layouts.3dm");
    let file = read(&data).unwrap();
    let reference: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(models().join("layouts.json")).unwrap()).unwrap();
    let point = |q: &serde_json::Value| DVec3::new(q[0].as_f64().unwrap(), q[1].as_f64().unwrap(), q[2].as_f64().unwrap());
    let ids: Vec<_> = file.views.iter().map(|v| (v.name.clone(), v.id.to_string())).collect();
    let rhino: Vec<_> = reference["views"].as_array().unwrap().iter().map(|v| (v["name"].as_str().unwrap().to_string(), v["id"].as_str().unwrap().to_string())).collect();
    assert_eq!(ids, rhino);
    let pages: Vec<_> = file.views.iter().filter_map(|v| v.page.map(|p| (v.name.clone(), v.id.to_string(), p.width, p.height, p.number as i64))).collect();
    let rhino: Vec<_> = reference["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["name"].as_str().unwrap().to_string(), p["id"].as_str().unwrap().to_string(), p["width"].as_f64().unwrap(), p["height"].as_f64().unwrap(), p["number"].as_i64().unwrap()))
        .collect();
    assert_eq!(pages, rhino);
    for r in reference["details"].as_array().unwrap() {
        let name = r["name"].as_str().unwrap();
        let o = file.objects.iter().find(|o| o.attributes.name == name).unwrap_or_else(|| panic!("{name}"));
        let Geometry::Detail { boundary, view, scale, locked } = &o.geometry else { panic!("{name}: not a detail") };
        let (lo, hi) = boundary.iter().fold((DVec3::INFINITY, DVec3::NEG_INFINITY), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
        let corners = r["corners"].as_array().unwrap();
        assert!(lo.distance(point(&corners[0])) < 1e-9 && hi.distance(point(&corners[1])) < 1e-9, "{name}: {lo} {hi}");
        assert_eq!(boundary.len(), 5, "{name}");
        assert_eq!((o.attributes.space, o.attributes.viewport.to_string()), (2, r["page"].as_str().unwrap().to_string()), "{name}");
        assert_eq!(view.id.to_string(), r["viewport"].as_str().unwrap(), "{name}");
        assert_eq!((view.perspective, *locked), (r["perspective"].as_bool().unwrap(), r["locked"].as_bool().unwrap()), "{name}");
        assert!(view.page.is_none(), "{name}");
        assert!(view.location.distance(point(&r["location"])) < 1e-6, "{name}");
        assert!(view.direction.distance(point(&r["direction"]).normalize()) < 1e-9, "{name}");
        assert!(view.up.distance(point(&r["up"]).normalize()) < 1e-9, "{name}");
        let f: Vec<f64> = r["frustum"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
        if view.perspective {
            // Rhino fits the live viewport's frustum to the detail on screen: the field of view
            // stored differs by a fraction of a percent
            assert!((view.fov_y - 2.0 * (0.5 * (f[3] - f[2]) / f[4]).atan()).abs() < 2e-3, "{name}: {}", view.fov_y);
        } else {
            assert!((view.height - (f[3] - f[2])).abs() < 1e-9, "{name}");
        }
        let rhino_scale = r["scale"].as_f64().unwrap();
        assert_eq!(*scale, (rhino_scale > 0.0).then_some(rhino_scale), "{name}");
    }
    for r in reference["page_objects"].as_array().unwrap() {
        let name = r["name"].as_str().unwrap();
        let a = &file.objects.iter().find(|o| o.attributes.name == name).unwrap_or_else(|| panic!("{name}")).attributes;
        assert_eq!((a.space, a.viewport.to_string()), (2, r["page"].as_str().unwrap().to_string()), "{name}");
    }
    assert_eq!(file.objects.iter().filter(|o| o.attributes.space == 2).count(), reference["page_objects"].as_array().unwrap().len());
}

/// The detail of McNeel's `V5/v5_example_file.3dm` (set `OPENNURBS_EXAMPLES` to the
/// `example_files` folder of an openNURBS checkout) as Rhino 9 reads it: its letter page, the
/// Top detail on it with its rectangle and scale, and the four leaders on the page (Rhino 5
/// attributes).
#[test]
fn the_detail_of_the_opennurbs_v5_example_is_read() {
    let Ok(folder) = std::env::var("OPENNURBS_EXAMPLES") else { return };
    let data = std::fs::read(std::path::Path::new(&folder).join("V5/v5_example_file.3dm")).unwrap();
    let file = read(&data).unwrap();
    let page = "c00c0f9e-8dcf-4bdb-83d9-09ec1eb413dd";
    let pages: Vec<_> = file.views.iter().filter_map(|v| v.page.map(|p| (v.name.as_str(), v.id.to_string(), p.width, p.height, p.number))).collect();
    assert_eq!(pages, [("Page 1", page.to_string(), 215.9, 279.4, 0)]);
    let on_page: Vec<_> = file.objects.iter().filter(|o| o.attributes.space == 2).collect();
    assert_eq!(on_page.len(), 5);
    assert!(on_page.iter().all(|o| o.attributes.viewport.to_string() == page));
    let detail = file.objects.iter().find(|o| o.attributes.id.to_string() == "09154fd0-8eae-416c-a2d8-17c339b6035e").unwrap();
    let Geometry::Detail { boundary, view, scale, locked } = &detail.geometry else { panic!("not a detail") };
    let (lo, hi) = boundary.iter().fold((DVec3::INFINITY, DVec3::NEG_INFINITY), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
    assert!(lo.distance(DVec3::new(10.0, 10.0, 0.0)) < 1e-9 && hi.distance(DVec3::new(205.9, 269.4, 0.0)) < 1e-9, "{lo} {hi}");
    assert_eq!((view.name.as_str(), view.perspective, *locked, *scale), ("Top", false, false, Some(1.2249496764269077)));
}

/// The units of the layouts (`page_units.3dm`, from make_page_units.py): a model in metres
/// whose layouts are in inches, as Rhino 9 reads it (`page_units.json`); a file that does not
/// say has the model's units on its layouts.
#[test]
fn layouts_have_their_own_units() {
    let data = load("page_units.3dm");
    let file = read(&data).unwrap();
    assert_eq!((file.unit, file.page_unit), ("metres", "inches"));
    assert_eq!((file.meters, file.page_meters), (1.0, 0.0254));
    let data = load("layouts.3dm");
    let file = read(&data).unwrap();
    assert_eq!((file.unit, file.page_unit), ("millimetres", "millimetres"));
}

/// A number Rhino's script wrote, as a number or as text.
fn number(j: &serde_json::Value) -> f64 {
    j.as_f64().or_else(|| j.as_str().and_then(|s| s.parse().ok())).unwrap_or(f64::NAN)
}

/// Whether a dictionary value is what RhinoCommon reads back (`[type, value]` of
/// make_objects.py): colours with their alpha (ours the transparency, 255 − alpha), planes by
/// origin and axes (ours with the z axis and equation after them).
fn same_value(v: &threedm_read::Value, j: &serde_json::Value) -> bool {
    use threedm_read::Value;
    let (ty, j) = (j[0].as_str().unwrap_or(""), &j[1]);
    match v {
        Value::Bool(b) => j.as_bool() == Some(*b),
        Value::Int(i) => number(j) == *i as f64,
        Value::Number(x) => (number(j) - x).abs() < 1e-9,
        Value::Uuid(u) => j.as_str().is_some_and(|s| s.eq_ignore_ascii_case(&u.to_string())),
        Value::String(s) => j.as_str() == Some(s),
        Value::Color([r, g, b, t]) => *j == serde_json::json!([r, g, b, 255 - t]),
        Value::Numbers(n) => {
            let a = j.as_array().unwrap();
            (a.len() == n.len() || ty == "Plane" && a.len() == 9) && a.iter().zip(n).all(|(a, b)| (number(a) - b).abs() < 1e-6)
        }
        Value::Array(items) => {
            let a = j.as_array().unwrap();
            let element = ty.trim_end_matches("[]");
            a.len() == items.len() && a.iter().zip(items).all(|(a, v)| same_value(v, &serde_json::json!([element, a])))
        }
        Value::Dictionary(d) => same_dictionary(d, j),
        Value::Object { class, bytes } => ty == "LineCurve" && *class == threedm_read::Uuid::parse("4ED7D4DB-E947-11d3-BFE5-0010830122F0") && *bytes > 0,
        Value::Raw(_) => false,
    }
}

fn same_dictionary(d: &threedm_read::Dictionary, j: &serde_json::Value) -> bool {
    let j = j.as_object().unwrap();
    d.entries.len() == j.len() && d.entries.iter().all(|e| j.get(&e.key).is_some_and(|v| same_value(&e.value, v)))
}

/// Plug-in data and rare objects (`objects.3dm`, from make_objects.py) as Rhino 9 reads them
/// (`objects.json`): user dictionaries of every value type and user text on objects (geometry
/// and attributes), a layer and a material, the document user text, the plug-in that wrote
/// each; a morph control on a curve, a cage and a cage morph by their control points and
/// extents.
#[test]
fn plugin_data_and_rare_objects_match_rhino() {
    use threedm_read::{Decoded, Owner};
    let data = load("objects.3dm");
    let file = read(&data).unwrap();
    let rhino: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(models().join("objects.json")).unwrap()).unwrap();
    let decoded = |owner: Owner| file.plugin_data.iter().filter(move |d| d.owner == owner).filter_map(|d| d.decoded.as_ref());
    let dictionary = |owner: Owner| decoded(owner).find_map(|d| if let Decoded::Dictionary(d) = d { Some(d.clone()) } else { None }).unwrap_or_default();
    let text = |owner: Owner| -> serde_json::Value {
        let pairs: serde_json::Map<String, serde_json::Value> = decoded(owner).filter_map(|d| if let Decoded::UserText(t) = d { Some(t.clone()) } else { None }).flatten().map(|(k, v)| (k, v.into())).collect();
        pairs.into()
    };
    // every user data entry of RhinoCommon's dictionaries names its plug-in and is kept whole
    let common = threedm_read::Uuid::parse("2544a64e-220d-4d65-b8d4-611bb57b46c7");
    let dictionaries: Vec<_> = file.plugin_data.iter().filter(|d| d.class == common).collect();
    assert_eq!(dictionaries.len(), 4);
    assert!(dictionaries.iter().all(|d| !d.plugin.is_nil() && d.version == (2, 2) && d.archive == 90 && !d.goo && d.bytes.len() > 100));
    for (table, records, key) in [("layers", &file.layers.iter().map(|l| l.name.clone()).collect::<Vec<_>>(), "layers"), ("materials", &file.materials.iter().map(|m| m.name.clone()).collect(), "materials")] {
        for expected in rhino[key].as_array().unwrap() {
            let index = records.iter().position(|n| n == expected["name"].as_str().unwrap()).unwrap();
            let owner = Owner::Table { table, index };
            assert!(same_dictionary(&dictionary(owner), &expected["dictionary"]), "{table} {index}");
            assert_eq!(text(owner), expected["user_text"], "{table} {index}");
        }
    }
    for expected in rhino["objects"].as_array().unwrap() {
        let name = expected["name"].as_str().unwrap();
        let index = file.objects.iter().position(|o| o.attributes.name == name).unwrap();
        for (attributes, dict, user) in [(true, "attributes_dictionary", "attributes_user_text"), (false, "geometry_dictionary", "geometry_user_text")] {
            let owner = Owner::Object { index, attributes };
            assert!(same_dictionary(&dictionary(owner), &expected[dict]), "{name}: {dict}");
            assert_eq!(text(owner), expected[user], "{name}: {user}");
        }
        // the extents of what is drawn
        let points: Vec<DVec3> = match &file.objects[index].geometry {
            Geometry::Lines(lines, _) => lines.iter().flatten().copied().collect(),
            Geometry::Drawing { lines, .. } => lines.iter().flatten().copied().collect(),
            _ => Vec::new(),
        };
        if name == "Morph curve" || name == "Edge curve" || name.starts_with("Cage") {
            // the morph curve by its curve, the cages by their control nets
            let points = if name == "Morph curve" { let Geometry::Lines(lines, _) = &file.objects[index].geometry else { panic!() }; lines[0].clone() } else { points };
            let (lo, hi) = points.iter().fold((DVec3::INFINITY, DVec3::NEG_INFINITY), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
            let b: Vec<f64> = expected["bbox"].as_array().unwrap().iter().map(number).collect();
            // a polyline within the display tolerance misses the extremes of a curve by a little
            let tol = if name == "Morph curve" { 0.05 } else { 1e-9 };
            assert!(lo.distance(DVec3::new(b[0], b[1], b[2])) < tol && hi.distance(DVec3::new(b[3], b[4], b[5])) < tol, "{name}: {lo} {hi} {b:?}");
        }
        // control points: of the morph curve its curve's, of the cages their grips
        let grips: Vec<[f64; 3]> = expected.get("grips").and_then(|g| g.as_array()).map(|g| g.iter().map(|p| [number(&p[0]), number(&p[1]), number(&p[2])]).collect()).unwrap_or_default();
        let control = file.morph_controls.iter().find(|m| m.object == index).map(|m| m.control.clone()).or_else(|| file.cages.iter().find(|c| c.0 == index).map(|c| c.1.clone()));
        if let Some(control) = control {
            let ours: Vec<[f64; 3]> = control.points.iter().map(|p| [p[0], p[1], p[2]]).collect();
            assert_eq!(ours.len(), grips.len(), "{name}");
            assert!(ours.iter().zip(&grips).all(|(a, b)| (0..3).all(|k| (a[k] - b[k]).abs() < 1e-9)), "{name}");
        }
        if let Some(m) = file.morph_controls.iter().find(|m| m.object == index) {
            assert_eq!((m.tolerance, m.quick_preview, m.preserve_structure), (number(&expected["tolerance"]), expected["quick_preview"] == true, expected["preserve_structure"] == true), "{name}");
            if let Some(curve) = expected["curve_points"].as_array().filter(|c| !c.is_empty()) {
                assert_eq!((m.kind, m.control.count), (1, [curve.len(), 1, 1]));
                assert_eq!(m.start.as_ref().map(|s| s.count), Some([curve.len(), 1, 1]));
            } else {
                // the cage morph deforms the sphere
                assert_eq!(m.kind, 3);
                let sphere = file.objects.iter().find(|o| o.attributes.name == "Captive sphere").unwrap().attributes.id;
                assert_eq!(m.captives, [sphere]);
            }
        }
    }
    // the kinds as Rhino names them
    let kind = |name: &str| match &file.objects.iter().find(|o| o.attributes.name == name).unwrap().geometry {
        Geometry::Lines(_, kind) | Geometry::Meshes(_, kind) | Geometry::Drawing { kind, .. } | Geometry::Points { kind, .. } => *kind,
        _ => "",
    };
    assert_eq!([kind("Cage"), kind("Morph curve"), kind("Edge curve")], ["Cage", "Morph control", "Curve"]);
    let Geometry::Drawing { meshes, .. } = &file.objects.iter().find(|o| o.attributes.name == "Cage").unwrap().geometry else { panic!() };
    assert_eq!(area(&file, &Geometry::Meshes(meshes.clone(), "Cage"), DMat4::IDENTITY, 0).round(), 2400.0);
    // the document's user text, and the user tables (render content as XML)
    let strings: Vec<(String, String)> = rhino["strings"].as_array().unwrap().iter().map(|p| (p[0].as_str().unwrap().to_string(), p[1].as_str().unwrap().to_string())).collect();
    assert_eq!(file.document_user_text, strings);
    let tables: Vec<String> = file.user_tables.iter().map(|t| t.plugin.to_string()).collect();
    for p in rhino["plugins"].as_array().unwrap() {
        assert!(tables.contains(&p["id"].as_str().unwrap().to_string()));
    }
    let rdk = file.user_tables.iter().find(|t| t.name == "Renderer Development Kit").unwrap();
    assert!(matches!(&rdk.decoded, Some(Decoded::Text(t)) if t.starts_with("<xml>") && t.ends_with("</xml>")));
}

/// No object of McNeel's example files (when `OPENNURBS_EXAMPLES` names their folder) is of a
/// class the reader does not know, and every user table is read.
#[test]
fn every_object_class_of_the_opennurbs_examples_is_known() {
    let Ok(folder) = std::env::var("OPENNURBS_EXAMPLES") else { return };
    let mut stack = vec![std::path::PathBuf::from(folder)];
    while let Some(dir) = stack.pop() {
        for path in std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()) {
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("3dm")) {
                let data = std::fs::read(&path).unwrap();
                let Ok(file) = read(&data) else { continue };
                assert!(!file.objects.iter().any(|o| matches!(o.geometry, Geometry::Skipped("other objects"))), "{}", path.display());
                assert!(!file.damaged.contains_key("user tables"), "{}", path.display());
            }
        }
    }
}

/// Rhino 8 drafting (`drafting8.3dm`, from make_drafting8.py) as Rhino 9 reads it
/// (`drafting8.json`): linetypes with a width in millimetres, in centimetres and without unit,
/// their caps, joins and taper; the section style table, layers and objects with a section
/// style of the table or their own, a clipping plane's label; the strokes of curves (by object
/// and by layer, with their print width); a hatch drawn with its background fill and boundary.
#[test]
fn rhino8_drafting_is_read_as_in_rhino() {
    use threedm_read::{LineCap, LineJoin, SectionStyle};
    let data = load("drafting8.3dm");
    let file = read(&data).unwrap();
    let reference: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(models().join("drafting8.json")).unwrap()).unwrap();
    assert!(reference["errors"].as_array().unwrap().is_empty());
    let color = |q: &serde_json::Value| -> [u8; 4] { std::array::from_fn(|i| q[i].as_u64().unwrap() as u8) };
    let number = |q: &serde_json::Value| q.as_f64().unwrap();
    let int = |q: &serde_json::Value| q.as_i64().unwrap();
    let caps = [LineCap::Round, LineCap::Flat, LineCap::Square];
    let joins = [LineJoin::Round, LineJoin::Miter, LineJoin::Bevel];

    let rhino = reference["linetypes"].as_array().unwrap();
    assert_eq!(file.linetypes.len(), rhino.len());
    for r in rhino {
        let name = r["name"].as_str().unwrap();
        let lt = file.linetypes.iter().find(|l| l.name == name).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(lt.index as i64, int(&r["index"]), "{name}");
        assert_eq!(lt.id.to_string(), r["id"].as_str().unwrap(), "{name}");
        assert_eq!(lt.segments, r["segments"].as_array().unwrap().iter().map(number).collect::<Vec<_>>(), "{name}");
        assert_eq!(lt.width, number(&r["width"]), "{name}");
        assert_eq!(lt.width_unit as i64, int(&r["width_units"]), "{name}");
        assert_eq!(lt.cap, caps[int(&r["cap"]) as usize], "{name}");
        assert_eq!(lt.join, joins[int(&r["join"]) as usize], "{name}");
        assert_eq!(lt.taper, r["taper"].as_array().unwrap().iter().map(|p| [number(&p[0]), number(&p[1])]).collect::<Vec<_>>(), "{name}");
        assert_eq!(lt.always_model_distances, r["always_model"].as_bool().unwrap(), "{name}");
    }

    // every field of a section style, as RhinoCommon names them
    let same_style = |s: &SectionStyle, r: &serde_json::Value| {
        let name = r["Name"].as_str().unwrap();
        assert_eq!(s.name, name);
        let mode = ["None", "Viewport", "SolidColor"].iter().position(|m| *m == r["BackgroundFillMode"].as_str().unwrap()).unwrap();
        assert_eq!(s.background_fill_mode as usize, mode, "{name}");
        assert_eq!(s.background_fill_color, color(&r["BackgroundFillColor"]), "{name}");
        assert_eq!(s.background_fill_print_color, color(&r["BackgroundFillPrintColor"]), "{name}");
        assert_eq!(s.boundary_visible, r["BoundaryVisible"].as_bool().unwrap(), "{name}");
        assert_eq!(s.boundary_color, color(&r["BoundaryColor"]), "{name}");
        assert_eq!(s.boundary_print_color, color(&r["BoundaryPrintColor"]), "{name}");
        assert_eq!(s.boundary_width_scale, number(&r["BoundaryWidthScale"]), "{name}");
        assert_eq!(s.boundary_print_width, number(&r["BoundaryPlotWeightMillimeters"]), "{name}");
        assert_eq!(s.boundary_linetype_index as i64, int(&r["BoundaryLinetypeIndex"]), "{name}");
        let rule = ["ClosedCurves", "SolidObjects"].iter().position(|m| *m == r["SectionFillRule"].as_str().unwrap()).unwrap();
        assert_eq!(s.fill_rule as usize, rule, "{name}");
        assert_eq!(s.hatch_index as i64, int(&r["HatchIndex"]), "{name}");
        assert_eq!(s.hatch_scale, number(&r["HatchScale"]), "{name}");
        assert!((s.hatch_rotation - number(&r["HatchRotationRadians"])).abs() < 1e-12, "{name}");
        assert_eq!(s.hatch_color, color(&r["HatchPatternColor"]), "{name}");
        assert_eq!(s.hatch_print_color, color(&r["HatchPatternPrintColor"]), "{name}");
        assert_eq!(s.hatch_print_width, number(&r["HatchPatternPlotWeightMillimeters"]), "{name}");
    };
    let same_own = |s: &Option<SectionStyle>, r: &serde_json::Value, what: &str| match (s, r) {
        (None, serde_json::Value::Null) => {}
        (Some(s), r) if !r.is_null() => same_style(s, r),
        _ => panic!("{what}: {s:?} against {r}"),
    };
    let rhino = reference["section_styles"].as_array().unwrap();
    assert_eq!(file.section_styles.len(), rhino.len());
    for r in rhino {
        let s = file.section_styles.iter().find(|s| s.name == r["Name"].as_str().unwrap()).unwrap();
        assert_eq!(s.index as i64, int(&r["Index"]));
        assert_eq!(s.id.to_string(), r["Id"].as_str().unwrap());
        same_style(s, r);
    }

    for r in reference["layers"].as_array().unwrap() {
        let name = r["name"].as_str().unwrap();
        let l = file.layers.iter().find(|l| l.name == name).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(l.linetype as i64, int(&r["linetype"]), "{name}");
        assert_eq!(l.plot_weight, number(&r["plot_weight"]), "{name}");
        assert_eq!(l.plot_color, color(&r["plot_color"]), "{name}");
        assert_eq!(l.section_style as i64, int(&r["section_style"]), "{name}");
        same_own(&l.custom_section_style, &r["custom_section_style"], name);
    }

    for r in reference["objects"].as_array().unwrap() {
        let name = r["name"].as_str().unwrap();
        let o = file.objects.iter().find(|o| o.attributes.name == name).unwrap_or_else(|| panic!("{name}"));
        let a = &o.attributes;
        assert_eq!(a.layer as i64, int(&r["layer"]), "{name}");
        assert_eq!((a.linetype as i64, a.linetype_source as i64), (int(&r["linetype"]), int(&r["linetype_source"])), "{name}");
        assert_eq!((a.plot_weight, a.plot_weight_source as i64), (number(&r["plot_weight"]), int(&r["plot_weight_source"])), "{name}");
        assert_eq!(a.section_source as i64, int(&r["section_source"]), "{name}");
        assert_eq!(a.section_style as i64, int(&r["section_style"]), "{name}");
        same_own(&a.custom_section_style, &r["custom_section_style"], name);
        assert_eq!(a.clipping_label as i64, int(&r["label_style"]), "{name}");
        assert_eq!(a.hatch_background.unwrap_or([255; 4]), color(&r["hatch_background"]), "{name}");
        assert_eq!(a.hatch_boundary, r["hatch_boundary"].as_bool().unwrap(), "{name}");
        // the stroke of a curve: its linetype's width, cap and join, its print width
        if r["type"] == "Curve" {
            let s = o.stroke.as_ref().unwrap_or_else(|| panic!("{name}"));
            let layer = file.layers.iter().find(|l| l.index == a.layer).unwrap();
            let index = if a.linetype_source == 1 { a.linetype } else { layer.linetype };
            let lt = file.linetypes.iter().find(|l| l.index == index).unwrap();
            assert_eq!((s.linetype, s.width, s.width_unit, s.cap, s.join), (index, lt.width, lt.width_unit, lt.cap, lt.join), "{name}");
            assert_eq!(s.taper, lt.taper, "{name}");
            let print = if a.plot_weight_source == 1 { a.plot_weight } else { layer.plot_weight };
            assert_eq!(s.print_width, print, "{name}");
        } else {
            assert!(o.stroke.is_none(), "{name}");
        }
    }
    // widths in model units (millimetres): 2 mm, 4 cm, none for a width without unit
    let stroke = |name: &str| file.objects.iter().find(|o| o.attributes.name == name).unwrap().stroke.clone().unwrap();
    assert!((stroke("Wide mm").model_width.unwrap() - 2.0).abs() < 1e-9);
    assert!((stroke("Wide model").model_width.unwrap() - 40.0).abs() < 1e-9);
    assert_eq!(stroke("Tapered").model_width, None);
    assert_eq!((stroke("Curve by layer").print_width, stroke("Curve by layer").print_color), (0.7, [255, 0, 0, 0]));

    // the hatch: its background fill behind its pattern, its boundary (a square 20 by 20)
    let hatch = file.objects.iter().find(|o| o.attributes.name == "Filled hatch").unwrap();
    let Geometry::Drawing { lines, meshes, kind } = &hatch.geometry else { panic!("{:?}", hatch.geometry) };
    assert_eq!(*kind, "Hatch");
    assert_eq!(meshes.len(), 1);
    assert_eq!(meshes[0].color, Some([200, 220, 255, 0]));
    assert_eq!(meshes[0].triangles, 2);
    let boundary: Vec<f64> = lines.iter().filter(|l| l.len() > 2).map(|l| l.windows(2).map(|w| w[0].distance(w[1])).sum()).collect();
    assert_eq!(boundary.len(), 1);
    assert!((boundary[0] - 80.0).abs() < 1e-9, "{boundary:?}");
    assert!(lines.len() > 5, "the pattern's lines too");

    // the hatch pattern table, by name and index, with the families of lines
    for r in reference["hatch_patterns"].as_array().unwrap() {
        let name = r["name"].as_str().unwrap();
        let p = file.hatch_patterns.iter().find(|p| p.name == name).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(p.index as i64, int(&r["index"]), "{name}");
        assert_eq!(p.solid, r["fill"] == "Solid", "{name}");
        assert!(!p.lines.is_empty() && p.lines.iter().all(|l| l.offset[1] != 0.0), "{name}: {:?}", p.lines);
    }
    assert_eq!(file.hatch_patterns.iter().find(|p| p.name == "Grid").unwrap().lines.len(), 2);
}

/// The document settings (`settings.3dm`, from make_settings.py) as Rhino 9 reads them
/// (`settings.json`): the earth anchor point and model base point, named and view construction
/// planes, mesh, grid and render settings, the current entries, named layer states, a snapshot,
/// a history record, document user text, the plug-in tables, and the fonts of the texts, the
/// dimension and the dimension styles.
#[test]
fn settings_are_read_as_in_rhino() {
    let data = load("settings.3dm");
    let file = read(&data).unwrap();
    let r: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(models().join("settings.json")).unwrap()).unwrap();
    let point = |q: &serde_json::Value| DVec3::new(q[0].as_f64().unwrap(), q[1].as_f64().unwrap(), q[2].as_f64().unwrap());
    let number = |q: &serde_json::Value| q.as_f64().unwrap();
    let int = |q: &serde_json::Value| q.as_i64().unwrap();
    let flag = |q: &serde_json::Value| q.as_bool().unwrap();
    let text = |q: &serde_json::Value| q.as_str().unwrap_or("").to_string();
    let color = |q: &serde_json::Value| -> [u8; 4] { std::array::from_fn(|i| q[i].as_u64().unwrap() as u8) };
    let s = &file.settings;
    assert!(file.damaged.is_empty(), "{:?}", file.damaged);

    // the earth anchor point and the model base point
    let e = s.earth_anchor.as_ref().unwrap();
    let re = &r["earth_anchor"];
    for (field, ours) in [("latitude", e.latitude), ("longitude", e.longitude), ("elevation", e.elevation)] {
        assert_eq!(ours, number(&re[field]), "{field}");
    }
    for (field, ours) in [("model_point", e.model_point), ("model_north", e.model_north), ("model_east", e.model_east)] {
        assert_eq!(ours, point(&re[field]), "{field}");
    }
    assert_eq!((e.name.as_str(), e.description.as_str()), (re["name"].as_str().unwrap(), re["description"].as_str().unwrap()));
    assert_eq!(e.coordinate_system as i64, int(&re["coordinate_system"]));
    assert_eq!(s.model_basepoint, point(&r["model_basepoint"]));
    assert_eq!(s.model_url, text(&r["model_url"]));

    // construction planes: named, and of each view
    let cplane = |ours: &threedm_read::ConstructionPlane, rhino: &serde_json::Value, what: &str| {
        let plane = &rhino["plane"];
        for (field, v) in [("origin", ours.origin), ("x", ours.x), ("y", ours.y)] {
            assert!(v.distance(point(&plane[field])) < 1e-12, "{what} {field}: {v} {}", plane[field]);
        }
        assert_eq!((ours.grid_spacing, ours.snap_spacing), (number(&rhino["grid_spacing"]), number(&rhino["snap_spacing"])), "{what}");
        assert_eq!((ours.grid_line_count as i64, ours.thick_frequency as i64), (int(&rhino["grid_line_count"]), int(&rhino["thick_frequency"])), "{what}");
        assert_eq!(ours.depth_buffered, flag(&rhino["depth_buffered"]), "{what}");
    };
    let named = r["named_cplanes"].as_array().unwrap();
    assert_eq!(s.named_cplanes.len(), named.len());
    for (ours, rhino) in s.named_cplanes.iter().zip(named) {
        assert_eq!(ours.name, text(&rhino["name"]));
        cplane(ours, rhino, &ours.name);
    }
    for view in r["views"].as_array().unwrap() {
        let id = view["id"].as_str().unwrap();
        let (_, ours) = s.view_cplanes.iter().find(|(v, _)| v.to_string() == id).unwrap_or_else(|| panic!("view {id}"));
        cplane(ours, &view["cplane"], view["name"].as_str().unwrap());
    }

    // mesh settings
    for (field, ours) in [("render_mesh", &s.render_mesh), ("analysis_mesh", &s.analysis_mesh), ("custom_render_mesh", &s.custom_render_mesh)] {
        let (m, rm) = (ours.as_ref().unwrap(), &r[field]);
        assert_eq!((m.texture_range as i64, m.jagged_seams, m.refine, m.simple_planes, m.compute_curvature), (int(&rm["texture_range"]), flag(&rm["jagged_seams"]), flag(&rm["refine_grid"]), flag(&rm["simple_planes"]), flag(&rm["compute_curvature"])), "{field}");
        assert_eq!((m.grid_min_count as i64, m.grid_max_count as i64), (int(&rm["grid_min_count"]), int(&rm["grid_max_count"])), "{field}");
        for (name, v) in [("grid_angle", m.grid_angle), ("grid_aspect_ratio", m.grid_aspect_ratio), ("grid_amplification", m.grid_amplification), ("tolerance", m.tolerance), ("relative_tolerance", m.relative_tolerance), ("minimum_edge_length", m.min_edge_length), ("maximum_edge_length", m.max_edge_length), ("refine_angle", m.refine_angle)] {
            assert_eq!(v, number(&rm[name]), "{field} {name}");
        }
    }

    // grid defaults, render settings, the current entries
    let (g, rg) = (s.grid.as_ref().unwrap(), &r["grid"]);
    assert_eq!((g.grid_spacing, g.snap_spacing), (number(&rg["grid_spacing"]), number(&rg["snap_spacing"])));
    assert_eq!((g.grid_line_count as i64, g.thick_frequency as i64), (int(&rg["grid_line_count"]), int(&rg["thick_frequency"])));
    assert_eq!((g.show_grid, g.show_axes, g.show_world_axes), (flag(&rg["show_grid"]), flag(&rg["show_axes"]), flag(&rg["show_world_axes"])));
    let (rs, rr) = (s.render.as_ref().unwrap(), &r["render"]);
    assert_eq!((rs.ambient, rs.background_color, rs.background_bottom), (color(&rr["ambient"]), color(&rr["background_top"]), color(&rr["background_bottom"])));
    let flags = [rs.use_hidden_lights, rs.depth_cue, rs.flat_shade, rs.render_backfaces, rs.render_points, rs.render_curves, rs.render_isoparams, rs.render_mesh_edges, rs.render_annotations, rs.scale_background, rs.transparent_background, !rs.custom_image_size];
    let names = ["use_hidden_lights", "depth_cue", "flat_shade", "render_backfaces", "render_points", "render_curves", "render_isoparams", "render_mesh_edges", "render_annotations", "scale_background", "transparent_background", "use_viewport_size"];
    for (ours, name) in flags.iter().zip(names) {
        assert_eq!(*ours, flag(&rr[name]), "{name}");
    }
    assert_eq!((rs.background_style as i64, rs.antialias as i64, rs.shadowmap as i64, rs.image_unit as i64, rs.rendering_source as i64), (int(&rr["background_style"]), int(&rr["antialias"]), int(&rr["shadowmap_level"]), int(&rr["image_unit"]), int(&rr["render_source"])));
    assert_eq!((rs.image_dpi, [rs.image_width as i64, rs.image_height as i64]), (number(&rr["image_dpi"]), [int(&rr["image_size"][0]), int(&rr["image_size"][1])]));
    assert_eq!((rs.named_view.clone(), rs.snapshot.clone(), rs.specific_viewport.clone()), (text(&rr["named_view"]), text(&rr["snapshot"]), text(&rr["specific_viewport"])));
    let (c, rc) = (&s.current, &r["current"]);
    for (field, ours) in [("layer", c.layer), ("material", c.material), ("linetype", c.linetype), ("text_style", c.text_style), ("dimension_style", c.dimension_style)] {
        assert_eq!(ours.to_string(), rc[field].as_str().unwrap(), "current {field}");
    }
    // Rhino gives the built-in Solid hatch pattern (not in the file's table) as nil
    assert_eq!(c.hatch_pattern.to_string(), "b319435d-86b6-4d89-972d-1f75fef0d950");
    assert_eq!(rc["hatch_pattern"].as_str().unwrap(), "00000000-0000-0000-0000-000000000000");
    // RhinoCommon gives these two colours with their alpha as stored (the others as opacity)
    let stored = |q: &serde_json::Value| -> [u8; 4] { std::array::from_fn(|i| if i == 3 { 255 - q[i].as_u64().unwrap() as u8 } else { q[i].as_u64().unwrap() as u8 }) };
    assert_eq!((c.color, c.plot_color, c.wire_density as i64), (stored(&rc["color"]), stored(&rc["plot_color"]), int(&rc["wire_density"])));
    assert_eq!(s.active_view.to_string(), r["active_view"].as_str().unwrap());
    assert_eq!((s.linetype_display_scale, s.save_texture_bitmaps, s.idef_link_update as i64), (number(&r["linetype_display_scale"]), flag(&r["save_texture_bitmaps"]), int(&r["idef_link_update"])));

    // the plug-in tables, the layer states, the snapshot, the document user text
    let mut tables: Vec<String> = file.user_tables.iter().map(|t| t.plugin.to_string()).filter(|p| p != "06f3218e-f5ec-4f6c-b74c-14583f0ed7bc").collect();
    let mut rhino: Vec<String> = r["plugin_data"].as_array().unwrap().iter().map(text).collect();
    tables.sort();
    rhino.sort();
    assert_eq!(tables, rhino);
    let states: Vec<&str> = file.layer_states.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(states, r["layer_states"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect::<Vec<_>>());
    let roofs = |state: usize| file.layer_states[state].layers.iter().find(|l| l.name == "Roofs").unwrap().visible;
    assert!(roofs(0) && !roofs(1));
    assert_eq!(file.snapshots.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Snap one"]);
    let strings: Vec<(String, String)> = r["strings"].as_array().unwrap().iter().map(|p| (text(&p[0]), text(&p[1]))).collect();
    assert_eq!(file.document_user_text, strings);

    // the history record of the extrusion: from the circle
    for h in r["history"].as_array().unwrap() {
        let record = file.history.iter().find(|x| x.descendants.iter().any(|d| d.to_string() == h["id"].as_str().unwrap())).expect("its history record");
        let parents: Vec<String> = h["parents"].as_array().unwrap().iter().map(text).collect();
        assert_eq!(record.antecedents.iter().map(|a| a.to_string()).collect::<Vec<_>>(), parents);
        assert!(!record.values.is_empty() && !record.command.is_nil());
    }
    assert_eq!(file.history.len(), r["history"].as_array().unwrap().len());

    // fonts: of each text and the dimension (its first character's), of each dimension style
    for t in r["texts"].as_array().unwrap() {
        let name = t["name"].as_str().unwrap();
        let o = file.objects.iter().find(|o| o.attributes.id.to_string() == t["id"].as_str().unwrap()).unwrap();
        let (f, rf) = (o.font.as_ref().unwrap_or_else(|| panic!("{name}: no font")), &t["first_char_font"]);
        assert_eq!((f.family.as_str(), f.bold, f.italic), (rf["family"].as_str().unwrap(), flag(&rf["bold"]), flag(&rf["italic"])), "{name}");
    }
    let styles = r["dim_styles"].as_array().unwrap();
    assert_eq!(file.dim_styles.len(), styles.len());
    for rs in styles {
        let d = file.dim_styles.iter().find(|d| d.id.to_string() == rs["id"].as_str().unwrap()).unwrap();
        assert_eq!((d.name.as_str(), d.index as i64), (rs["name"].as_str().unwrap(), int(&rs["index"])));
        let (f, rf) = (d.font.as_ref().unwrap(), &rs["font"]);
        for (field, ours) in [("family", &f.family), ("face", &f.face), ("postscript", &f.postscript), ("logfont", &f.logfont)] {
            assert_eq!(ours, rf[field].as_str().unwrap(), "{} {field}", d.name);
        }
        assert_eq!((f.weight as i64, f.style as i64, f.bold, f.italic, f.underlined, f.strikeout), (int(&rf["weight"]), int(&rf["style"]), flag(&rf["bold"]), flag(&rf["italic"]), flag(&rf["underlined"]), flag(&rf["strikeout"])), "{}", d.name);
    }
}

/// The settings, fonts and plug-in tables of McNeel's openNURBS example files (not in this
/// repository: set `OPENNURBS_EXAMPLES` to the `example_files` folder of an openNURBS
/// checkout): no font table, history record or user table is damaged; every file from Rhino 4
/// on has its render mesh settings; the dimension styles of Rhino 4 and 5 files have the font
/// of their text style.
#[test]
fn settings_of_the_opennurbs_examples_are_read() {
    let Ok(folder) = std::env::var("OPENNURBS_EXAMPLES") else { return };
    let mut files = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(folder)];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("3dm")) {
                files.push(path);
            }
        }
    }
    let (mut fonts, mut styles_with_fonts, mut texts_with_fonts, mut tables) = (0, 0, 0, 0);
    for path in &files {
        let data = std::fs::read(path).unwrap();
        let Ok(file) = read(&data) else { continue };
        for table in ["fonts", "history records", "user tables", "dimension styles"] {
            assert!(!file.damaged.contains_key(table), "{} {table}", path.display());
        }
        if file.versions.archive >= 4 {
            assert!(file.settings.render_mesh.is_some(), "{}", path.display());
        }
        fonts += file.fonts.len();
        styles_with_fonts += file.dim_styles.iter().filter(|d| d.font.as_ref().is_some_and(|f| !f.family.is_empty())).count();
        texts_with_fonts += file.objects.iter().filter(|o| o.font.is_some()).count();
        tables += file.user_tables.len();
        eprintln!("{}: {} fonts, {} styles, {} texts with a font, {} user tables", path.display(), file.fonts.len(), file.dim_styles.len(), file.objects.iter().filter(|o| o.font.is_some()).count(), file.user_tables.len());
    }
    eprintln!("{fonts} fonts, {styles_with_fonts} styles with a font, {texts_with_fonts} texts with a font, {tables} user tables");
    assert!(fonts > 0 && styles_with_fonts > 0 && texts_with_fonts > 0 && tables > 0);
}

/// A caller that has the fonts of the file sets each text in its own: given a font file for
/// "Times New Roman" only (here one that is no font, so the text comes out empty), the Times
/// texts take it and the others the default font.
#[test]
fn texts_are_set_in_the_font_the_caller_gives_for_theirs() {
    fn resolve(font: &threedm_read::Font) -> Option<&'static [u8]> {
        (font.family == "Times New Roman").then_some(b"not a font".as_slice())
    }
    let data = load("settings.3dm");
    let options = threedm_read::Options { resolve_font: Some(resolve), ..threedm_read::Options::default() };
    let file = threedm_read::read_with(&data, &options).unwrap();
    let mut seen = (0, 0);
    for o in &file.objects {
        let (Some(font), Geometry::Meshes(meshes, "Text")) = (&o.font, &o.geometry) else { continue };
        let drawn = meshes.iter().any(|m| !m.faces.is_empty());
        if font.family == "Times New Roman" {
            assert!(!drawn, "a Times text set in the default font");
            seen.0 += 1;
        } else {
            assert!(drawn, "{} not set", font.family);
            seen.1 += 1;
        }
    }
    assert!(seen.0 > 0 && seen.1 > 0, "{seen:?}");
}

/// Rendering data (`render.3dm`, from make_render.py) as Rhino 9 reads it (`render.json`): the
/// render settings with the environments, ground plane, sun (at a date and place), skylight,
/// linear workflow, dithering, safe frame and render channels; every render material,
/// environment and texture with its parameters; the render materials of the materials, layers
/// and objects; the decal on a box and the mesh modifiers of four objects.
#[test]
fn rendering_is_read_as_in_rhino() {
    use threedm_read::RenderValue;
    let data = load("render.3dm");
    let file = read(&data).unwrap();
    assert!(file.damaged.is_empty(), "{:?}", file.damaged);
    let r: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(models().join("render.json")).unwrap()).unwrap();
    let f = |v: &serde_json::Value| v.as_f64().unwrap();
    let b = |v: &serde_json::Value| v.as_bool().unwrap();
    let i = |v: &serde_json::Value| v.as_i64().unwrap();
    let s = |v: &serde_json::Value| v.as_str().unwrap_or_default().to_string();
    let near = |a: f64, v: &serde_json::Value, what: &str| assert!((a - f(v)).abs() < 1e-9, "{what}: {a} {v}");
    let arr = |v: &serde_json::Value| v.as_array().unwrap().iter().map(f).collect::<Vec<_>>();
    // System.Drawing colours: alpha 255 opaque; openNURBS: 0 opaque
    let rgba = |c: [u8; 4]| vec![c[0] as f64, c[1] as f64, c[2] as f64, 255.0 - c[3] as f64];

    let rs = &file.render;
    let x = &r["render_settings"];
    assert_eq!(rs.custom_image_size, !b(&x["use_viewport_size"]));
    assert_eq!([rs.image_width as i64, rs.image_height as i64], [i(&x["image_size"][0]), i(&x["image_size"][1])]);
    near(rs.image_dpi, &x["image_dpi"], "dpi");
    assert_eq!(rs.image_unit as i64, i(&x["image_unit_system"]));
    assert_eq!(rgba(rs.ambient), arr(&x["ambient_light"]));
    assert_eq!(rgba(rs.background_color), arr(&x["background_color_top"]));
    assert_eq!(rgba(rs.background_bottom_color), arr(&x["background_color_bottom"]));
    assert_eq!(rs.background_style as i64, i(&x["background_style"]));
    for (ours, field) in [
        (rs.use_hidden_lights, "use_hidden_lights"),
        (rs.depth_cue, "depth_cue"),
        (rs.flat_shade, "flat_shade"),
        (rs.render_backfaces, "render_backfaces"),
        (rs.render_points, "render_points"),
        (rs.render_curves, "render_curves"),
        (rs.render_isoparams, "render_isoparams"),
        (rs.render_mesh_edges, "render_mesh_edges"),
        (rs.render_annotations, "render_annotations"),
        (rs.scale_background_to_fit, "scale_background_to_fit"),
        (rs.transparent_background, "transparent_background"),
    ] {
        assert_eq!(ours, b(&x[field]), "{field}");
    }
    assert_eq!((rs.antialias as i64, rs.shadowmap as i64, rs.rendering_source as i64), (i(&x["antialias_level"]), i(&x["shadowmap_level"]), i(&x["render_source"])));
    assert_eq!((rs.named_view.clone(), rs.snapshot.clone(), rs.specific_viewport.clone()), (s(&x["named_view"]), s(&x["snapshot"]), s(&x["specific_viewport"])));

    let env = &r["environments"];
    assert_eq!(rs.background_environment.to_string(), s(&env["background"]["id"]));
    assert_eq!((rs.reflection_environment_on, rs.reflection_environment.to_string()), (b(&env["reflection"]["override"]), s(&env["reflection"]["id"])));
    assert_eq!((rs.skylight.environment_on, rs.skylight.environment.to_string()), (b(&env["skylighting"]["override"]), s(&env["skylighting"]["id"])));

    let g = (&rs.ground_plane, &r["ground_plane"]);
    assert_eq!((g.0.on, g.0.auto_altitude, g.0.shadow_only, g.0.show_underside), (b(&g.1["enabled"]), b(&g.1["auto_altitude"]), b(&g.1["shadow_only"]), b(&g.1["show_underside"])));
    near(g.0.altitude, &g.1["altitude"], "ground plane altitude");
    assert_eq!(g.0.material.to_string(), s(&g.1["material"]));
    assert_eq!((g.0.texture_offset.to_vec(), g.0.texture_size.to_vec()), (arr(&g.1["texture_offset"]), arr(&g.1["texture_size"])));
    near(g.0.texture_rotation, &g.1["texture_rotation"], "ground plane rotation");
    assert_eq!((g.0.texture_offset_locked, g.0.texture_size_locked), (b(&g.1["texture_offset_locked"]), b(&g.1["texture_size_locked"])));

    let (sun, y) = (&rs.sun, &r["sun"]);
    assert_eq!((sun.on, sun.manual, sun.daylight_saving_on, sun.daylight_saving_minutes as i64), (b(&y["enabled"]), b(&y["manual_control"]), b(&y["daylight_saving_on"]), i(&y["daylight_saving_minutes"])));
    for (ours, field) in [(sun.azimuth, "azimuth"), (sun.altitude, "altitude"), (sun.north, "north"), (sun.latitude, "latitude"), (sun.longitude, "longitude"), (sun.time_zone, "time_zone"), (sun.intensity, "intensity")] {
        near(ours, &y[field], field);
    }
    let local = arr(&y["local"]);
    assert_eq!([sun.year as f64, sun.month as f64, sun.day as f64], local[..3]);
    assert!((sun.hours - (local[3] + local[4] / 60.0 + local[5] / 3600.0)).abs() < 1e-9);
    let diffuse: Vec<u8> = arr(&y["light_diffuse"]).iter().map(|c| *c as u8).collect();
    assert_eq!(sun.color.map(|c| (c * 255.0).round() as u8)[..3], diffuse[..3]);
    let direction = arr(&y["light_direction"]);
    assert!((0..3).all(|k| (sun.direction[k] - direction[k]).abs() < 1e-9), "{:?} {direction:?}", sun.direction);

    let k = &r["skylight"];
    assert_eq!(rs.skylight.on, b(&k["enabled"]));
    near(rs.skylight.shadow_intensity, &k["shadow_intensity"], "skylight shadow");
    let lw = &r["linear_workflow"];
    assert_eq!((rs.linear_workflow.pre_process, rs.linear_workflow.pre_process, rs.linear_workflow.post_process), (b(&lw["pre_process_colors"]), b(&lw["pre_process_textures"]), b(&lw["post_process_gamma_on"])));
    assert!((rs.linear_workflow.gamma - f(&lw["post_process_gamma"])).abs() < 1e-6);
    let method = if s(&r["dithering"]["method_name"]) == "FloydSteinberg" { "floyd-steinberg" } else { "simple-noise" };
    assert_eq!((rs.dithering.on, rs.dithering.method), (b(&r["dithering"]["enabled"]), method));
    let (sf, z) = (&rs.safe_frame, &r["safe_frame"]);
    assert_eq!((sf.on, sf.perspective_only, sf.field_grid_on, sf.live_frame_on), (b(&z["enabled"]), b(&z["perspective_only"]), b(&z["fields_on"]), b(&z["live_frame_on"])));
    for (frame, name) in [(&sf.action_frame, "action"), (&sf.title_frame, "title")] {
        assert_eq!((frame.on, frame.linked), (b(&z[format!("{name}_frame_on")]), b(&z[format!("{name}_frame_linked")])), "{name}");
        near(frame.x_scale, &z[format!("{name}_frame_x_scale")], name);
        near(frame.y_scale, &z[format!("{name}_frame_y_scale")], name);
    }
    let channels = &r["render_channels"];
    assert_eq!(rs.render_channels_custom, i(&channels["mode"]) != 0);
    assert_eq!(rs.render_channels.iter().map(|u| u.to_string()).collect::<Vec<_>>(), channels["custom"].as_array().unwrap().iter().map(s).collect::<Vec<_>>());
    assert_eq!(rs.post_effects.len(), r["post_effects"].as_array().unwrap().len());

    // render content: every one, with its parameters
    let contents = r["render_content"].as_array().unwrap();
    assert_eq!(file.render_content.len(), contents.len());
    let mut compared = 0;
    for c in contents {
        let ours = file.render_content.iter().find(|o| o.id.to_string() == s(&c["id"])).unwrap_or_else(|| panic!("{}", c["id"]));
        let name = s(&c["name"]);
        assert_eq!((ours.kind, ours.name.as_str()), (s(&c["kind"]).as_str(), name.as_str()));
        assert_eq!((ours.type_name.clone(), ours.type_id.to_string()), (s(&c["type_name"]), s(&c["type_id"])), "{name}");
        assert_eq!((ours.render_engine.to_string(), ours.plug_in.to_string(), ours.group.to_string()), (s(&c["render_engine_id"]), s(&c["plug_in_id"]), s(&c["group_id"])), "{name}");
        assert_eq!((ours.notes.clone(), ours.tags.clone(), ours.child_slot.clone()), (s(&c["notes"]), s(&c["tags"]), s(&c["child_slot_name"])), "{name}");
        assert_eq!((ours.hidden, ours.reference, ours.auto_delete), (b(&c["hidden"]), b(&c["reference"]), b(&c["auto_delete"])), "{name}");
        let parent = if ours.parent.is_nil() { String::new() } else { ours.parent.to_string() };
        assert_eq!(parent, s(&c["parent"]), "{name}");
        assert_eq!(ours.children.iter().map(|u| u.to_string()).collect::<Vec<_>>(), c["children"].as_array().unwrap().iter().map(s).collect::<Vec<_>>(), "{name}");
        for (param, v) in c["parameters"].as_object().unwrap() {
            let Some(mine) = ours.parameter(param) else { panic!("{name} {param}") };
            if v.is_null() {
                // a parameter without a value (an empty child slot)
                assert!(matches!(mine, RenderValue::Null | RenderValue::Text(_)), "{name} {param}: {mine:?}");
                continue;
            }
            let value = &v["value"];
            match (v["type"].as_str().unwrap_or_default(), mine) {
                ("Bool", RenderValue::Bool(m)) => assert_eq!(*m, b(value), "{name} {param}"),
                ("Integer", RenderValue::Integer(m)) => assert_eq!(*m, i(value), "{name} {param}"),
                ("Double" | "Float", RenderValue::Number(m)) => assert!((m - f(value)).abs() <= 1e-6 * f(value).abs().max(1.0), "{name} {param}: {m} {value}"),
                ("Color", RenderValue::Color(m)) => assert!(m.iter().zip(arr(value)).all(|(a, b)| (*a as f64 - b).abs() < 1e-6), "{name} {param}: {m:?} {value}"),
                ("Vector3d" | "Vector2d" | "Point4d", RenderValue::Numbers(m)) => assert!(m.iter().zip(arr(value)).all(|(a, b)| (a - b).abs() < 1e-9), "{name} {param}: {m:?} {value}"),
                // RhinoCommon gives no text of a string variant
                ("String", RenderValue::Text(_) | RenderValue::Null) => continue,
                (t, m) => panic!("{name} {param}: {t} {m:?}"),
            }
            compared += 1;
        }
    }
    assert!(compared > 200, "{compared}");
    for kind in ["material", "environment", "texture"] {
        assert!(file.render_content.iter().any(|c| c.kind == kind), "{kind}");
    }
    // a child in a slot: the bitmap texture of the custom material
    let tiles = file.render_content.iter().find(|c| c.name == "Tiles").unwrap();
    let tile = file.render_content.iter().find(|c| c.parent == tiles.id).unwrap();
    assert_eq!((tile.child_slot.as_str(), tile.kind), ("bitmap-texture", "texture"));

    // the materials Rhino made for the render materials, and who uses them
    for m in r["materials"].as_array().unwrap() {
        let ours = file.materials.iter().find(|o| o.name == s(&m["name"])).unwrap();
        assert_eq!((ours.id.to_string(), ours.render_material.to_string()), (s(&m["id"]), s(&m["render_material"])));
        assert_eq!(file.render_material(ours.index).map(|c| c.name.as_str()), Some(ours.name.as_str()));
    }
    for l in r["layers"].as_array().unwrap() {
        assert_eq!(file.layers.iter().find(|o| o.name == s(&l["name"])).unwrap().material as i64, i(&l["material"]));
    }
    let used: HashMap<String, String> = file.objects.iter().map(|o| (o.attributes.name.clone(), file.render_material(file.object_material(o)).map(|c| c.name.clone()).unwrap_or_default())).collect();
    assert_eq!((used["Decal box"].as_str(), used["Displaced box"].as_str(), used["Thick surface"].as_str(), used["Piped curve"].as_str()), ("Brushed copper", "Wall plaster", "Tiles", ""));

    // decals and mesh modifiers
    for o in r["objects"].as_array().unwrap() {
        let name = s(&o["name"]);
        let a = &file.objects.iter().find(|x| x.attributes.id.to_string() == s(&o["id"])).unwrap().attributes;
        assert_eq!((a.material as i64, a.material_source as i64), (i(&o["material"]), i(&o["material_source"])), "{name}");
        let decals = o["decals"].as_array().unwrap();
        assert_eq!(a.decals.len(), decals.len(), "{name}");
        for (d, x) in a.decals.iter().zip(decals) {
            assert_eq!((d.texture.to_string(), d.mapping.clone(), d.projection.clone()), (s(&x["texture"]), s(&x["mapping_name"]).to_lowercase(), s(&x["projection_name"]).to_lowercase()));
            assert_eq!((d.map_to_inside, d.visible), (b(&x["map_to_inside"]), b(&x["visible"])));
            for (ours, field) in [(d.transparency, "transparency"), (d.height, "height"), (d.radius, "radius")] {
                near(ours, &x[field], field);
            }
            assert_eq!((d.origin.to_vec(), d.vector_up.to_vec(), d.vector_across.to_vec()), (arr(&x["origin"]), arr(&x["vector_up"]), arr(&x["vector_across"])));
            assert_eq!((d.horizontal_sweep.to_vec(), d.vertical_sweep.to_vec(), d.uv_bounds.to_vec()), (arr(&x["horz_sweep"]), arr(&x["vert_sweep"]), arr(&x["uv_bounds"])));
        }
        let m = &o["mesh_modifiers"];
        let mm = &a.mesh_modifiers;
        assert_eq!(mm.displacement.is_some(), m.get("displacement").is_some(), "{name}");
        if let (Some(d), Some(x)) = (&mm.displacement, m.get("displacement")) {
            assert_eq!((d.on, d.texture.to_string(), d.mapping_channel as i64), (b(&x["on"]), s(&x["texture"]), i(&x["channel"])));
            let ints = [d.sweep_pitch, d.refine_steps, d.face_limit, d.fairing, d.memory_limit, d.sweep_formula].map(i64::from);
            assert_eq!(ints, ["sweep_pitch", "refine_steps", "face_limit", "fairing", "memory_limit", "sweep_formula"].map(|k| i(&x[k])));
            assert_eq!((d.face_limit_on, d.fairing_on), (b(&x["face_limit_on"]), b(&x["fairing_on"])));
            for (ours, field) in [(d.black_point, "black_point"), (d.white_point, "white_point"), (d.refine_sensitivity, "refine_sensitivity"), (d.post_weld_angle, "post_weld_angle")] {
                near(ours, &x[field], field);
            }
        }
        assert_eq!(mm.edge_softening.is_some(), m.get("edge_softening").is_some(), "{name}");
        if let (Some(e), Some(x)) = (&mm.edge_softening, m.get("edge_softening")) {
            assert_eq!((e.on, e.chamfer, e.faceted, e.force_softening), (b(&x["on"]), b(&x["chamfer"]), b(&x["faceted"]), b(&x["force_softening"])));
            near(e.softening, &x["softening"], "softening");
            near(e.edge_angle_threshold, &x["edge_angle_threshold"], "threshold");
        }
        assert_eq!(mm.thickening.is_some(), m.get("thickening").is_some(), "{name}");
        if let (Some(t), Some(x)) = (&mm.thickening, m.get("thickening")) {
            assert_eq!((t.on, t.solid, t.offset_only, t.both_sides), (b(&x["on"]), b(&x["solid"]), b(&x["offset_only"]), b(&x["both_sides"])));
            near(t.distance, &x["distance"], "distance");
        }
        assert_eq!(mm.curve_piping.is_some(), m.get("curve_piping").is_some(), "{name}");
        if let (Some(p), Some(x)) = (&mm.curve_piping, m.get("curve_piping")) {
            assert_eq!((p.on, p.faceted, p.segments as i64, p.accuracy as i64, p.cap_type.to_string()), (b(&x["on"]), b(&x["faceted"]), i(&x["segments"]), i(&x["accuracy"]), s(&x["cap_name"]).to_lowercase()));
            near(p.radius, &x["radius"], "radius");
        }
        assert_eq!(mm.shut_lining.is_some(), m.get("shut_lining").is_some(), "{name}");
        if let (Some(l), Some(x)) = (&mm.shut_lining, m.get("shut_lining")) {
            assert_eq!((l.on, l.faceted, l.auto_update, l.force_update, l.curves.len()), (b(&x["on"]), b(&x["faceted"]), b(&x["auto_update"]), b(&x["force_update"]), x["curves"].as_array().unwrap().len()));
        }
    }
    let with = |f: fn(&threedm_read::MeshModifiers) -> bool| file.objects.iter().filter(|o| f(&o.attributes.mesh_modifiers)).count();
    assert_eq!([with(|m| m.displacement.is_some()), with(|m| m.edge_softening.is_some()), with(|m| m.thickening.is_some()), with(|m| m.curve_piping.is_some()), with(|m| m.shut_lining.is_some())], [1; 5]);
    assert_eq!(file.objects.iter().map(|o| o.attributes.decals.len()).sum::<usize>(), 1);
}
