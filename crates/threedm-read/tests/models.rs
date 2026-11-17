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
