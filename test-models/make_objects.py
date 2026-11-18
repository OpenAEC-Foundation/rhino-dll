# -*- coding: utf-8 -*-
"""Plug-in data and rare object classes (objects.3dm): objects, a layer and a material with
user dictionaries (RhinoCommon's ArchivableDictionary) of every value type, user text on
geometry and attributes, document user text; a morph control on a curve, a cage
(NurbsCage) and a cage morph (CageEdit), a brep edge added as a curve. What Rhino reads back:
objects.json."""
import os
import json
import System
import Rhino
import Rhino.Geometry as rg
import scriptcontext as sc
import rhinoscriptsyntax as rs

OUT = os.environ.get("MODELS_OUT")
LOG = []


def value(v):
    """A dictionary value as JSON: [type name, value]."""
    t = v.GetType().Name if hasattr(v, "GetType") else type(v).__name__
    if isinstance(v, Rhino.Collections.ArchivableDictionary):
        return ["ArchivableDictionary", dictionary(v)]
    if isinstance(v, bool):
        return [t, v]
    if isinstance(v, (int, float)):
        return [t, v]
    if isinstance(v, str):
        return [t, v]
    if isinstance(v, System.Guid):
        return [t, str(v)]
    if isinstance(v, System.Drawing.Color):
        return [t, [int(v.R), int(v.G), int(v.B), int(v.A)]]
    if isinstance(v, (rg.Point3d, rg.Vector3d)):
        return [t, [v.X, v.Y, v.Z]]
    if isinstance(v, (rg.Point2d, rg.Vector2d)):
        return [t, [v.X, v.Y]]
    if isinstance(v, (rg.Point3f, rg.Vector3f)):
        return [t, [float(v.X), float(v.Y), float(v.Z)]]
    if isinstance(v, rg.Point4d):
        return [t, [v.X, v.Y, v.Z, v.W]]
    if isinstance(v, rg.Interval):
        return [t, [v.T0, v.T1]]
    if isinstance(v, rg.Line):
        return [t, [v.FromX, v.FromY, v.FromZ, v.ToX, v.ToY, v.ToZ]]
    if isinstance(v, rg.Plane):
        return [t, [v.OriginX, v.OriginY, v.OriginZ, v.XAxis.X, v.XAxis.Y, v.XAxis.Z, v.YAxis.X, v.YAxis.Y, v.YAxis.Z]]
    if isinstance(v, rg.BoundingBox):
        return [t, [v.Min.X, v.Min.Y, v.Min.Z, v.Max.X, v.Max.Y, v.Max.Z]]
    if isinstance(v, rg.Transform):
        return [t, [v[i, j] for i in range(4) for j in range(4)]]
    if isinstance(v, rg.Ray3d):
        return [t, [v.Position.X, v.Position.Y, v.Position.Z, v.Direction.X, v.Direction.Y, v.Direction.Z]]
    if isinstance(v, rg.GeometryBase):
        return [t, str(v.ObjectType)]
    for attrs in (("X", "Y", "Width", "Height"), ("Width", "Height"), ("X", "Y")):
        if all(hasattr(v, a) for a in attrs):
            return [t, [float(getattr(v, a)) for a in attrs]]
    try:
        return [t, [value(x)[1] for x in v]]
    except Exception:
        return [t, str(v)]


def dictionary(d):
    out = {}
    for k in d.Keys:
        out[k] = value(d[k])
    return out


def fill(d):
    """Values of every type RhinoCommon's ArchivableDictionary takes."""
    def put(key, v):
        try:
            d.Set(key, v)
        except Exception as e:
            LOG.append("%s: %s" % (key, e))
    put("bool", True)
    put("byte", System.Byte(200))
    put("sbyte", System.SByte(-100))
    put("short", System.Int16(-1234))
    put("ushort", System.UInt16(54321))
    put("int", System.Int32(-123456))
    put("uint", System.UInt32(3000000000))
    put("long", System.Int64(-9876543210))
    put("float", System.Single(1.5))
    put("double", 3.25)
    put("string", "hello wereld")
    put("guid", System.Guid("0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0"))
    put("color", System.Drawing.Color.FromArgb(128, 10, 20, 30))
    put("point2i", System.Drawing.Point(3, -4))
    put("point2f", System.Drawing.PointF(1.25, -2.5))
    put("rect4i", System.Drawing.Rectangle(1, 2, 30, 40))
    put("rect4f", System.Drawing.RectangleF(1.5, 2.5, 3.5, 4.5))
    put("size2i", System.Drawing.Size(640, 480))
    put("size2f", System.Drawing.SizeF(6.5, 4.5))
    put("interval", rg.Interval(-1.0, 2.0))
    put("point2d", rg.Point2d(1.0, 2.0))
    put("point3d", rg.Point3d(1.0, 2.0, 3.0))
    put("point4d", rg.Point4d(1.0, 2.0, 3.0, 4.0))
    put("vector2d", rg.Vector2d(0.5, -0.5))
    put("vector3d", rg.Vector3d(0.0, 0.0, 1.0))
    put("bbox", rg.BoundingBox(rg.Point3d(0, 0, 0), rg.Point3d(1, 2, 3)))
    put("ray", rg.Ray3d(rg.Point3d(1, 1, 1), rg.Vector3d(0, 1, 0)))
    put("xform", rg.Transform.Translation(10, 20, 30))
    put("plane", rg.Plane(rg.Point3d(1, 2, 3), rg.Vector3d(0, 0, 1)))
    put("line", rg.Line(rg.Point3d(0, 0, 0), rg.Point3d(5, 6, 7)))
    put("point3f", rg.Point3f(1.5, 2.5, 3.5))
    put("vector3f", rg.Vector3f(0.25, 0.5, 0.75))
    put("ints", System.Array[System.Int32]([1, 2, 3]))
    put("doubles", System.Array[System.Double]([0.5, 1.5]))
    put("strings", System.Array[System.String](["a", "bc"]))
    put("bools", System.Array[System.Boolean]([True, False, True]))
    put("guids", System.Array[System.Guid]([System.Guid("11111111-2222-3333-4444-555555555555")]))
    put("floats", System.Array[System.Single]([0.25, 0.75]))
    try:
        d.Set.Overloads[str, System.Collections.Generic.IEnumerable[System.Byte]]("bytes", System.Array[System.Byte]([1, 2, 255]))
    except Exception as e:
        LOG.append("bytes: %s" % e)
    put("shorts", System.Array[System.Int16]([-1, 2]))
    put("geometry", rg.LineCurve(rg.Point3d(0, 0, 0), rg.Point3d(1, 0, 0)))
    nested = Rhino.Collections.ArchivableDictionary()
    nested.Set("inner int", 7)
    nested.Set("inner string", "deep")
    deeper = Rhino.Collections.ArchivableDictionary()
    deeper.Set("level", 2)
    nested.Set("deeper", deeper)
    put("nested", nested)


def user_text(o):
    keys = o.GetUserStrings()
    return dict((k, o.GetUserString(k)) for k in keys.AllKeys)


def main():
    doc = sc.doc
    for i in [o.Id for o in doc.Objects]:
        doc.Objects.Delete(i, True)
    # document user text
    doc.Strings.SetString("Project", "Objects test")
    doc.Strings.SetString("Section", "Entry", "Sectioned value")
    # a layer and a material with dictionaries and user text
    layer = Rhino.DocObjects.Layer()
    layer.Name = "Data layer"
    layer.UserDictionary.Set("layer int", 42)
    layer.UserDictionary.Set("layer string", "on the layer")
    layer.SetUserString("layer key", "layer value")
    li = doc.Layers.Add(layer)
    mat = Rhino.DocObjects.Material()
    mat.Name = "Data material"
    mat.UserDictionary.Set("material double", 0.125)
    mat.SetUserString("material key", "material value")
    mi = doc.Materials.Add(mat)

    a = Rhino.DocObjects.ObjectAttributes()
    a.Name = "Dict attributes"
    a.LayerIndex = li
    fill(a.UserDictionary)
    a.SetUserString("attr key", "attr value")
    doc.Objects.AddPoint(rg.Point3d(0, 0, 0), a)

    line = rg.LineCurve(rg.Point3d(0, 10, 0), rg.Point3d(10, 10, 0))
    line.UserDictionary.Set("geometry int", 5)
    line.UserDictionary.Set("geometry point", rg.Point3d(4, 5, 6))
    line.SetUserString("geometry key", "geometry value")
    b = Rhino.DocObjects.ObjectAttributes()
    b.Name = "Dict geometry"
    b.MaterialIndex = mi
    b.MaterialSource = Rhino.DocObjects.ObjectMaterialSource.MaterialFromObject
    doc.Objects.AddCurve(line, b)

    # a morph control on a curve
    c0 = rg.NurbsCurve.Create(False, 3, [rg.Point3d(0, 30, 0), rg.Point3d(10, 30, 0), rg.Point3d(20, 30, 0), rg.Point3d(30, 30, 0)])
    c1 = rg.NurbsCurve.Create(False, 3, [rg.Point3d(0, 30, 0), rg.Point3d(10, 35, 0), rg.Point3d(20, 25, 5), rg.Point3d(30, 30, 0)])
    try:
        mc = rg.MorphControl(c0, c1)
        attr = Rhino.DocObjects.ObjectAttributes()
        attr.Name = "Morph curve"
        doc.Objects.AddMorphControl(mc, attr)
    except Exception as e:
        LOG.append("morph curve: %s" % e)

    # a cage: the Cage command, then CageEdit on a sphere with its own cage
    before = set(o.Id for o in doc.Objects)
    rs.Command("_-Cage 40,0,0 60,20,0 20 _Enter", False)
    for o in doc.Objects:
        if o.Id not in before:
            o.Attributes.Name = "Cage"
            o.CommitChanges()
    sphere = doc.Objects.AddSphere(rg.Sphere(rg.Point3d(80, 10, 10), 8))
    so = doc.Objects.FindId(sphere)
    so.Attributes.Name = "Captive sphere"
    so.CommitChanges()
    before = set(o.Id for o in doc.Objects)
    rs.UnselectAllObjects()
    rs.SelectObject(sphere)
    rs.Command("_-CageEdit _BoundingBox _World _XPointCount=4 _YPointCount=4 _ZPointCount=4 _XDegree=3 _YDegree=3 _ZDegree=3 _Enter _Global _Enter", False)
    for o in doc.Objects:
        if o.Id not in before:
            o.Attributes.Name = "Cage morph " + str(o.ObjectType)
            o.CommitChanges()
    rs.UnselectAllObjects()

    # a polysurface edge added as a curve
    box = rg.Box(rg.Plane.WorldXY, rg.Interval(0, 10), rg.Interval(-30, -20), rg.Interval(0, 10)).ToBrep()
    attr = Rhino.DocObjects.ObjectAttributes()
    attr.Name = "Edge curve"
    doc.Objects.AddCurve(box.Edges[0], attr)

    doc.WriteFile(os.path.join(OUT, "objects.3dm"), Rhino.FileIO.FileWriteOptions())
    f = Rhino.FileIO.File3dm.Read(os.path.join(OUT, "objects.3dm"))
    out = {"log": LOG, "objects": [], "layers": [], "materials": [], "strings": [], "plugins": []}
    try:
        for k in range(f.Strings.Count):
            out["strings"].append([f.Strings.GetKey(k), f.Strings.GetValue(k)])
    except Exception as e:
        LOG.append("strings: %s" % e)
    for l in f.AllLayers:
        out["layers"].append({"name": l.Name, "dictionary": dictionary(l.UserDictionary), "user_text": user_text(l)})
    for m in f.AllMaterials:
        out["materials"].append({"name": m.Name, "dictionary": dictionary(m.UserDictionary), "user_text": user_text(m)})
    try:
        for p in f.PlugInData:
            out["plugins"].append({"id": str(p.PlugInId)})
    except Exception as e:
        LOG.append("plugin data: %s" % e)
    for o in f.Objects:
        g = o.Geometry
        at = o.Attributes
        rec = {
            "name": at.Name,
            "id": str(o.Id),
            "type": str(g.ObjectType),
            "class": g.GetType().Name,
            "attributes_dictionary": dictionary(at.UserDictionary),
            "geometry_dictionary": dictionary(g.UserDictionary),
            "attributes_user_text": user_text(at),
            "geometry_user_text": user_text(g),
        }
        bb = g.GetBoundingBox(True)
        rec["bbox"] = [bb.Min.X, bb.Min.Y, bb.Min.Z, bb.Max.X, bb.Max.Y, bb.Max.Z]
        if isinstance(g, rg.MorphControl):
            rec["tolerance"] = g.SpaceMorphTolerance
            rec["quick_preview"] = g.QuickPreview
            rec["preserve_structure"] = g.PreserveStructure
            if g.Curve:
                rec["curve_points"] = [[p.Location.X, p.Location.Y, p.Location.Z] for p in g.Curve.Points]
            if g.Surface:
                rec["surface_points"] = [[p.Location.X, p.Location.Y, p.Location.Z] for p in g.Surface.Points]
        if isinstance(g, rg.Curve):
            rec["length"] = g.GetLength()
        out["objects"].append(rec)
    # the control points of the cages, by their grips
    for o in doc.Objects:
        if o.ObjectType in (Rhino.DocObjects.ObjectType.Cage, Rhino.DocObjects.ObjectType.MorphControl):
            try:
                o.GripsOn = True
                grips = o.GetGrips()
                pts = [[gp.CurrentLocation.X, gp.CurrentLocation.Y, gp.CurrentLocation.Z] for gp in grips] if grips else []
                for rec in out["objects"]:
                    if rec["id"] == str(o.Id):
                        rec["grips"] = pts
                o.GripsOn = False
            except Exception as e:
                LOG.append("grips: %s" % e)
    with open(os.path.join(OUT, "objects.json"), "w") as fh:
        json.dump(out, fh, indent=1)


try:
    main()
except Exception:
    import traceback
    with open(os.path.join(OUT, "objects_error.txt"), "w") as fh:
        fh.write(traceback.format_exc() + "\n" + "\n".join(LOG))
