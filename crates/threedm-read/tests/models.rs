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
