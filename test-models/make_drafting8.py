# -*- coding: utf-8 -*-
"""Rhino 8 drafting (drafting8.3dm): linetypes with a width (in millimetres and in model units),
line caps and joins, a taper; section styles in the section style table, used by a layer, by
an object and by a clipping plane, an object's own section style; a hatch with a background
fill and its boundary drawn. What Rhino reads back (the file opened again): drafting8.json."""
import os
import json
import math
import System
import Rhino
import Rhino.Geometry as rg
import Rhino.DocObjects as rd
import scriptcontext as sc

OUT = os.environ.get("MODELS_OUT")
errors = []


def tried(what, f):
    try:
        return f()
    except Exception as e:
        errors.append("%s: %s" % (what, e))
        return None


def color(c):
    if c is None:
        return None
    if c == System.Drawing.Color.Empty:
        return None
    return [int(c.R), int(c.G), int(c.B), 255 - int(c.A)]


def linetype(name, segments, width, units, cap, join, taper=None, model=False):
    lt = rd.Linetype()
    lt.Name = name
    for length, solid in segments:
        lt.AppendSegment(length, solid)
    tried(name + " width", lambda: setattr(lt, "Width", width))
    tried(name + " units", lambda: setattr(lt, "WidthUnits", units))
    tried(name + " cap", lambda: setattr(lt, "LineCapStyle", cap))
    tried(name + " join", lambda: setattr(lt, "LineJoinStyle", join))
    if taper:
        tried(name + " taper", lambda: lt.SetTaper(taper[0], rg.Point2d(taper[1][0], taper[1][1]), taper[2]))
    if model:
        tried(name + " model", lambda: setattr(lt, "AlwaysModelDistances", True))
    return sc.doc.Linetypes.Add(lt)


def section_style(name, fill_mode, fill, boundary, width_scale, rule, hatch, scale, rotation, hatch_color, visible=True):
    s = rd.SectionStyle()
    s.Name = name
    tried(name + " fill mode", lambda: setattr(s, "BackgroundFillMode", fill_mode))
    tried(name + " fill", lambda: setattr(s, "BackgroundFillColor", fill))
    tried(name + " boundary", lambda: setattr(s, "BoundaryColor", boundary))
    tried(name + " visible", lambda: setattr(s, "BoundaryVisible", visible))
    tried(name + " width scale", lambda: setattr(s, "BoundaryWidthScale", width_scale))
    tried(name + " rule", lambda: setattr(s, "SectionFillRule", rule))
    tried(name + " hatch", lambda: setattr(s, "HatchIndex", hatch))
    tried(name + " hatch scale", lambda: setattr(s, "HatchScale", scale))
    tried(name + " rotation", lambda: setattr(s, "HatchRotationRadians", rotation))
    tried(name + " hatch colour", lambda: setattr(s, "HatchPatternColor", hatch_color))
    return s


def style_json(s):
    if s is None:
        return None
    out = {}
    for p in ["Name", "Index", "Id", "BackgroundFillMode", "BackgroundFillColor", "BackgroundFillPrintColor", "BoundaryColor", "BoundaryPrintColor", "BoundaryVisible", "BoundaryWidthScale", "BoundaryLinetypeIndex", "BoundaryPlotWeightMillimeters", "SectionFillRule", "HatchIndex", "HatchScale", "HatchRotationRadians", "HatchPatternColor", "HatchPatternPrintColor", "HatchPatternPlotWeightMillimeters"]:
        v = tried("read " + p, lambda: getattr(s, p))
        if isinstance(v, System.Drawing.Color):
            v = color(v)
        elif isinstance(v, System.Guid):
            v = str(v)
        elif v is not None and not isinstance(v, (bool, int, float, str)):
            v = str(v)
        out[p] = v
    return out


def main():
    doc = sc.doc
    for i in [o.Id for o in doc.Objects]:
        doc.Objects.Delete(i, True)
    doc.ModelUnitSystem = Rhino.UnitSystem.Millimeters
    U = Rhino.UnitSystem
    cap, join = rd.LineCapStyle, rd.LineJoinStyle
    wide_mm = linetype("Wide mm", [(5.0, True), (2.0, False)], 2.0, U.Millimeters, cap.Flat, join.Miter)
    wide_model = linetype("Wide model", [(1.0, True)], 4.0, U.Centimeters, cap.Square, join.Bevel)
    tapered = linetype("Tapered", [(1.0, True)], 1.0, getattr(U, "None"), cap.Round, join.Round, taper=(0.5, (0.3, 3.0), 1.5), model=True)
    # curves with corners, by object
    def zigzag(y):
        return rg.Polyline([rg.Point3d(0, y, 0), rg.Point3d(30, y + 10, 0), rg.Point3d(60, y, 0), rg.Point3d(90, y + 10, 0)]).ToNurbsCurve()
    for k, (name, index) in enumerate([("Wide mm", wide_mm), ("Wide model", wide_model), ("Tapered", tapered)]):
        a = rd.ObjectAttributes()
        a.Name = name
        a.LinetypeIndex = index
        a.LinetypeSource = rd.ObjectLinetypeSource.LinetypeFromObject
        a.PlotWeightSource = rd.ObjectPlotWeightSource.PlotWeightFromObject
        a.PlotWeight = 0.25 * (k + 1)
        doc.Objects.AddCurve(zigzag(-20.0 * k), a)
    # a layer with a wide linetype, a print width and a section style
    def pattern(name):
        found = doc.HatchPatterns.FindName(name)
        if found:
            return found.Index
        return doc.HatchPatterns.Add(getattr(rd.HatchPattern.Defaults, name))
    hatch = pattern("Hatch2")
    grid = pattern("Grid")
    concrete = section_style("Concrete", rd.SectionBackgroundFillMode.SolidColor, System.Drawing.Color.FromArgb(200, 200, 190), System.Drawing.Color.FromArgb(40, 40, 40), 2.0, rd.ObjectSectionFillRule.SolidObjects, hatch, 2.0, math.radians(45), System.Drawing.Color.FromArgb(90, 90, 90))
    steel = section_style("Steel", getattr(rd.SectionBackgroundFillMode, "None"), System.Drawing.Color.Empty, System.Drawing.Color.FromArgb(0, 0, 255), 1.5, rd.ObjectSectionFillRule.ClosedCurves, grid, 0.5, 0.0, System.Drawing.Color.FromArgb(0, 128, 0), visible=False)
    # print colours and widths, and a linetype for its boundary (Rhino 9)
    tried("steel fill print", lambda: setattr(steel, "BackgroundFillPrintColor", System.Drawing.Color.FromArgb(250, 250, 250)))
    tried("steel boundary print", lambda: setattr(steel, "BoundaryPrintColor", System.Drawing.Color.FromArgb(0, 0, 100)))
    tried("steel hatch print", lambda: setattr(steel, "HatchPatternPrintColor", System.Drawing.Color.FromArgb(0, 60, 0)))
    tried("steel boundary weight", lambda: setattr(steel, "BoundaryPlotWeightMillimeters", 0.35))
    tried("steel hatch weight", lambda: setattr(steel, "HatchPatternPlotWeightMillimeters", 0.18))
    tried("steel boundary linetype", lambda: setattr(steel, "BoundaryLinetypeIndex", wide_mm))
    concrete_index = tried("add Concrete", lambda: doc.SectionStyles.Add(concrete))
    steel_index = tried("add Steel", lambda: doc.SectionStyles.Add(steel))
    layer = rd.Layer()
    layer.Name = "Walls"
    layer.LinetypeIndex = wide_mm
    layer.PlotWeight = 0.7
    layer.PlotColor = System.Drawing.Color.FromArgb(255, 0, 0)
    if concrete_index is not None:
        tried("layer section style", lambda: setattr(layer, "SectionStyleIndex", concrete_index))
    walls = doc.Layers.Add(layer)
    own = rd.Layer()
    own.Name = "Own section"
    tried("layer custom section", lambda: own.SetCustomSectionStyle(section_style("Layer own", rd.SectionBackgroundFillMode.SolidColor, System.Drawing.Color.FromArgb(255, 240, 200), System.Drawing.Color.FromArgb(120, 60, 0), 3.0, rd.ObjectSectionFillRule.ClosedCurves, hatch, 1.0, 0.0, System.Drawing.Color.FromArgb(10, 10, 10))))
    own_layer = doc.Layers.Add(own)
    # a box on the walls layer: its sections by layer
    a = rd.ObjectAttributes()
    a.Name = "Wall by layer"
    a.LayerIndex = walls
    doc.Objects.AddBrep(rg.Box(rg.Plane.WorldXY, rg.Interval(0, 40), rg.Interval(40, 50), rg.Interval(0, 30)).ToBrep(), a)
    a = rd.ObjectAttributes()
    a.Name = "Curve by layer"
    a.LayerIndex = walls
    doc.Objects.AddCurve(zigzag(-60.0), a)
    # a box with the steel section style by object
    a = rd.ObjectAttributes()
    a.Name = "Column by object"
    tried("object source", lambda: setattr(a, "SectionAttributesSource", rd.ObjectSectionAttributesSource.FromObject))
    if steel_index is not None:
        tried("object section style", lambda: setattr(a, "SectionStyleIndex", steel_index))
    doc.Objects.AddBrep(rg.Box(rg.Plane.WorldXY, rg.Interval(50, 60), rg.Interval(40, 50), rg.Interval(0, 30)).ToBrep(), a)
    # a box with its own section style
    a = rd.ObjectAttributes()
    a.Name = "Beam own style"
    tried("own source", lambda: setattr(a, "SectionAttributesSource", rd.ObjectSectionAttributesSource.FromObject))
    tried("own section style", lambda: a.SetCustomSectionStyle(section_style("Own", rd.SectionBackgroundFillMode.Viewport, System.Drawing.Color.FromArgb(10, 200, 10), System.Drawing.Color.FromArgb(200, 10, 10), 4.0, rd.ObjectSectionFillRule.SolidObjects, grid, 3.0, math.radians(30), System.Drawing.Color.FromArgb(1, 2, 3))))
    doc.Objects.AddBrep(rg.Box(rg.Plane.WorldXY, rg.Interval(70, 80), rg.Interval(40, 50), rg.Interval(0, 30)).ToBrep(), a)
    # a box on the layer with its own section style; one whose sections follow the clipping plane
    a = rd.ObjectAttributes()
    a.Name = "On own section layer"
    a.LayerIndex = own_layer
    doc.Objects.AddBrep(rg.Box(rg.Plane.WorldXY, rg.Interval(90, 100), rg.Interval(40, 50), rg.Interval(0, 30)).ToBrep(), a)
    a = rd.ObjectAttributes()
    a.Name = "From sectioner"
    tried("sectioner source", lambda: setattr(a, "SectionAttributesSource", rd.ObjectSectionAttributesSource.FromSectioner))
    doc.Objects.AddBrep(rg.Box(rg.Plane.WorldXY, rg.Interval(110, 120), rg.Interval(40, 50), rg.Interval(0, 30)).ToBrep(), a)
    # a clipping plane with the concrete style and a label
    a = rd.ObjectAttributes()
    a.Name = "Section A"
    tried("clip source", lambda: setattr(a, "SectionAttributesSource", rd.ObjectSectionAttributesSource.FromObject))
    if concrete_index is not None:
        tried("clip section style", lambda: setattr(a, "SectionStyleIndex", concrete_index))
    tried("clip label", lambda: setattr(a, "ClippingPlaneLabelStyle", rd.SectionLabelStyle.TextDotFromName))
    top = [v for v in doc.Views if v.ActiveViewport.Name == "Top"][0].ActiveViewport.Id
    plane = rg.Plane(rg.Point3d(0, 0, 15), rg.Vector3d(0, 0, -1))
    tried("clipping plane", lambda: doc.Objects.AddClippingPlane(plane, 150, 80, [top], a))
    # a hatch with a background fill, its boundary drawn
    c = rd.ObjectAttributes()
    c.Name = "Filled hatch"
    tried("hatch fill", lambda: setattr(c, "HatchBackgroundFillColor", System.Drawing.Color.FromArgb(200, 220, 255)))
    tried("hatch boundary", lambda: setattr(c, "HatchBoundaryVisible", True))
    square = rg.Polyline([rg.Point3d(0, -100, 0), rg.Point3d(20, -100, 0), rg.Point3d(20, -80, 0), rg.Point3d(0, -80, 0), rg.Point3d(0, -100, 0)]).ToNurbsCurve()
    h = rg.Hatch.Create(square, hatch, 0, 1, 0.001)
    doc.Objects.AddHatch(h[0], c)
    path = os.path.join(OUT, "drafting8.3dm")
    doc.WriteFile(path, Rhino.FileIO.FileWriteOptions())

    # what Rhino reads back
    d = Rhino.RhinoDoc.OpenHeadless(path)
    out = {"errors": errors, "linetypes": [], "section_styles": [], "layers": [], "objects": []}
    for lt in d.Linetypes:
        segs = []
        for k in range(lt.SegmentCount):
            length, solid = lt.GetSegment(k)
            segs.append(length if solid else -length)
        taper = tried("taper", lambda: [[p.X, p.Y] for p in (lt.GetTaperPoints() or [])])
        out["linetypes"].append({
            "index": lt.Index, "name": lt.Name, "id": str(lt.Id), "segments": segs,
            "width": tried("w", lambda: lt.Width), "width_units": tried("u", lambda: int(lt.WidthUnits)),
            "cap": tried("c", lambda: int(lt.LineCapStyle)), "join": tried("j", lambda: int(lt.LineJoinStyle)),
            "taper": taper, "always_model": tried("m", lambda: lt.AlwaysModelDistances),
        })
    for s in tried("styles", lambda: list(d.SectionStyles)) or []:
        out["section_styles"].append(style_json(s))
    out["hatch_patterns"] = [{"index": p.Index, "name": p.Name, "fill": str(p.FillType)} for p in d.HatchPatterns]
    for l in d.Layers:
        out["layers"].append({
            "index": l.Index, "name": l.Name, "linetype": l.LinetypeIndex, "plot_weight": l.PlotWeight,
            "plot_color": color(l.PlotColor),
            "section_style": tried("lss", lambda: l.SectionStyleIndex),
            "custom_section_style": style_json(tried("lcss", lambda: l.GetCustomSectionStyle())),
        })
    for o in d.Objects:
        at = o.Attributes
        out["objects"].append({
            "name": at.Name, "type": str(o.ObjectType), "layer": at.LayerIndex,
            "linetype": at.LinetypeIndex, "linetype_source": int(at.LinetypeSource),
            "plot_weight": at.PlotWeight, "plot_weight_source": int(at.PlotWeightSource),
            "section_source": tried("src", lambda: int(at.SectionAttributesSource)),
            "section_style": tried("ss", lambda: at.SectionStyleIndex),
            "custom_section_style": style_json(tried("css", lambda: at.GetCustomSectionStyle())),
            "label_style": tried("label", lambda: int(at.ClippingPlaneLabelStyle)),
            "hatch_background": tried("bg", lambda: color(at.HatchBackgroundFillColor)),
            "hatch_boundary": tried("bv", lambda: at.HatchBoundaryVisible),
        })
    out["errors"] = errors
    with open(os.path.join(OUT, "drafting8.json"), "w") as fh:
        json.dump(out, fh, indent=1)
    d.Dispose()


try:
    main()
except Exception:
    import traceback
    with open(os.path.join(OUT, "drafting8_error.txt"), "w") as fh:
        fh.write(traceback.format_exc() + "\n" + "\n".join(errors))
