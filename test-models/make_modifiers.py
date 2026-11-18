#! python3
# -*- coding: utf-8 -*-
"""Mesh modifiers (modifiers.3dm): one object per modifier and setting. Curve piping on curves
(caps none, flat, box and dome; smooth and faceted; an open, a closed and a polyline curve),
thickening of surfaces and a mesh (solid, both sides, offset only), edge softening of boxes
(fillet, chamfer, faceted), displacement of a plane by a PNG next to the file (modifiers_ramp.png)
and by a texture whose file is missing. With what Rhino 9 shows for each object
(modifiers.json): the meshes of RhinoObject.RenderMeshes, with the modifiers applied, their
area, signed volume, bounding box and vertex and face counts, and those of the plain render
mesh. MOD_DUMP=1 also writes every mesh (modifiers_dump.json, not committed)."""
import os
import json
import traceback
import System
import Rhino
import Rhino.Geometry as rg
import Rhino.Render as rr
import scriptcontext as sc

OUT = os.environ.get("MODELS_OUT")
DUMP = os.environ.get("MOD_DUMP") == "1"
NOTES = []
RAMP = "modifiers_ramp.png"


def note(what):
    NOTES.append(what)


def progress(what):
    if os.environ.get("MOD_PROGRESS") == "1":
        with open(os.path.join(OUT, "modifiers_progress.txt"), "a") as fh:
            fh.write(what + "
")


def ramp_png(path):
    """A 64 x 64 grey ramp: black at the left (u = 0), white at the right."""
    bmp = System.Drawing.Bitmap(64, 64)
    for x in range(64):
        g = int(round(x * 255.0 / 63.0))
        c = System.Drawing.Color.FromArgb(255, g, g, g)
        for y in range(64):
            bmp.SetPixel(x, y, c)
    bmp.Save(path, System.Drawing.Imaging.ImageFormat.Png)


def bitmap_texture(doc, name, path):
    sim = rr.SimulatedTexture(doc)
    sim.Filename = path
    t = rr.RenderTexture.NewBitmapTexture(sim, doc)
    t.BeginChange(rr.RenderContent.ChangeContexts.Program)
    t.Name = name
    t.EndChange()
    doc.RenderTextures.Add(t)
    return t


def build(doc):
    for i in [o.Id for o in doc.Objects]:
        doc.Objects.Delete(i, True)
    ids = rr.CustomRenderMeshes.MeshProviderIds
    P = {"piping": ids.CurvePiping, "thickening": ids.Thickening, "softening": ids.EdgeSoftening,
         "displacement": ids.Displacement, "shutlining": ids.Shutlining}
    objs = []

    def add(name, oid, provider, values):
        progress("add " + name)
        o = doc.Objects.FindId(oid)
        for k, v in values:
            try:
                o.SetCustomRenderMeshParameter(P[provider], k, v)
            except Exception as e:
                note("%s %s: %s" % (name, k, e))
        o.CommitChanges()
        objs.append((name, oid))

    def attrs(name):
        a = Rhino.DocObjects.ObjectAttributes()
        a.Name = name
        return a

    def wave(y):
        return rg.Curve.CreateInterpolatedCurve([rg.Point3d(0, y, 0), rg.Point3d(5, y + 4, 2), rg.Point3d(10, y, 4), rg.Point3d(15, y + 5, 1)], 3)

    pipes = [("pipe none", "none", 0.5, 8, False), ("pipe flat", "flat", 0.5, 8, False),
             ("pipe box", "box", 0.4, 12, True), ("pipe dome", "dome", 0.6, 16, False)]
    for k, (name, cap, r, seg, faceted) in enumerate(pipes):
        oid = doc.Objects.AddCurve(wave(10.0 * k), attrs(name))
        add(name, oid, "piping", [("on", True), ("radius", r), ("segments", seg), ("weld", not faceted), ("accuracy", 50), ("cap-type", cap)])
    oid = doc.Objects.AddCurve(rg.Circle(rg.Plane(rg.Point3d(30, 0, 0), rg.Vector3d.ZAxis), 4.0).ToNurbsCurve(), attrs("pipe circle"))
    add("pipe circle", oid, "piping", [("on", True), ("radius", 0.3), ("segments", 10), ("weld", True), ("accuracy", 50), ("cap-type", "flat")])
    pl = rg.Polyline([rg.Point3d(30, 20, 0), rg.Point3d(38, 20, 0), rg.Point3d(38, 26, 0), rg.Point3d(38, 26, 5)])
    oid = doc.Objects.AddPolyline(pl, attrs("pipe polyline"))
    add("pipe polyline", oid, "piping", [("on", True), ("radius", 0.4), ("segments", 6), ("weld", False), ("accuracy", 50), ("cap-type", "flat")])
    oid = doc.Objects.AddCurve(wave(50.0), attrs("pipe accurate"))
    add("pipe accurate", oid, "piping", [("on", True), ("radius", 0.5), ("segments", 8), ("weld", True), ("accuracy", 100), ("cap-type", "none")])

    def plane_srf(x, y, w, h, z=0.0):
        return rg.NurbsSurface.CreateFromCorners(rg.Point3d(x, y, z), rg.Point3d(x + w, y, z), rg.Point3d(x + w, y + h, z), rg.Point3d(x, y + h, z))

    thick = [("thick solid", plane_srf(60, 0, 10, 8), 0.5, True, False, False),
             ("thick both", plane_srf(60, 20, 10, 8), 0.6, True, False, True),
             ("thick offset", plane_srf(60, 40, 10, 8), 0.5, False, True, False),
             ("thick open", plane_srf(60, 60, 10, 8), 0.5, False, False, False)]
    for name, srf, d, solid, offset_only, both in thick:
        oid = doc.Objects.AddSurface(srf, attrs(name))
        add(name, oid, "thickening", [("on", True), ("distance", d), ("solid", solid), ("offset-only", offset_only), ("both-sides", both)])
    curved = rg.NurbsSurface.CreateFromCorners(rg.Point3d(80, 0, 0), rg.Point3d(90, 0, 0), rg.Point3d(90, 10, 3), rg.Point3d(80, 10, 3))
    oid = doc.Objects.AddSurface(curved, attrs("thick sloped"))
    add("thick sloped", oid, "thickening", [("on", True), ("distance", 0.4), ("solid", True), ("offset-only", False), ("both-sides", False)])
    mesh = rg.Mesh.CreateFromPlane(rg.Plane(rg.Point3d(85, 25, 0), rg.Vector3d.ZAxis), rg.Interval(-5, 5), rg.Interval(-4, 4), 5, 4)
    oid = doc.Objects.AddMesh(mesh, attrs("thick mesh"))
    add("thick mesh", oid, "thickening", [("on", True), ("distance", 0.3), ("solid", True), ("offset-only", False), ("both-sides", False)])

    def box(x, y, s=4.0):
        return rg.Box(rg.Plane.WorldXY, rg.Interval(x, x + s), rg.Interval(y, y + s), rg.Interval(0, s)).ToBrep()

    soft = [("soft fillet", 0.5, False, False), ("soft chamfer", 0.5, True, False), ("soft faceted", 0.8, False, True)]
    for k, (name, r, chamfer, faceted) in enumerate(soft):
        oid = doc.Objects.AddBrep(box(100 + 10 * k, 0), attrs(name))
        add(name, oid, "softening", [("on", True), ("softening", r), ("chamfer", chamfer), ("unweld", faceted), ("edge-threshold", 5.0), ("force-softening", False)])
    oid = doc.Objects.AddMesh(rg.Mesh.CreateFromBox(rg.Box(rg.Plane.WorldXY, rg.Interval(130, 136), rg.Interval(0, 3), rg.Interval(0, 2)), 1, 1, 1), attrs("soft mesh"))
    add("soft mesh", oid, "softening", [("on", True), ("softening", 0.4), ("chamfer", False), ("unweld", False), ("edge-threshold", 5.0), ("force-softening", False)])

    ramp_path = os.path.join(OUT, RAMP)
    ramp_png(ramp_path)
    ramp = bitmap_texture(doc, "Ramp", ramp_path)
    missing = bitmap_texture(doc, "Missing", os.path.join(OUT, "modifiers_missing.png"))
    oid = doc.Objects.AddSurface(plane_srf(0, 80, 10, 10), attrs("disp ramp"))
    add("disp ramp", oid, "displacement", [("on", True), ("texture", ramp.Id), ("channel", 1), ("black-point", 0.0), ("white-point", 1.0)])
    oid = doc.Objects.AddSurface(plane_srf(20, 80, 10, 10), attrs("disp missing"))
    add("disp missing", oid, "displacement", [("on", True), ("texture", missing.Id), ("channel", 1), ("black-point", 0.0), ("white-point", 1.0)])
    return objs


def tri_faces(m):
    out = []
    for i in range(m.Faces.Count):
        f = m.Faces[i]
        out.append((f.A, f.B, f.C))
        if f.IsQuad:
            out.append((f.A, f.C, f.D))
    return out


def stats(meshes):
    """Area, signed volume (divergence theorem), bounding box and counts of some meshes."""
    area = 0.0
    vol = 0.0
    lo = [1e300] * 3
    hi = [-1e300] * 3
    nv = nf = 0
    closed = True
    for m in meshes:
        nv += m.Vertices.Count
        nf += m.Faces.Count
        closed = closed and m.IsClosed
        p = [m.Vertices.Point3dAt(i) for i in range(m.Vertices.Count)]
        for q in p:
            for k, x in enumerate((q.X, q.Y, q.Z)):
                lo[k] = min(lo[k], x)
                hi[k] = max(hi[k], x)
        for a, b, c in tri_faces(m):
            A, B, C = p[a], p[b], p[c]
            u = B - A
            v = C - A
            area += 0.5 * rg.Vector3d.CrossProduct(u, v).Length
            vol += (A.X * (B.Y * C.Z - B.Z * C.Y) - A.Y * (B.X * C.Z - B.Z * C.X) + A.Z * (B.X * C.Y - B.Y * C.X)) / 6.0
    return {"area": area, "volume": vol, "closed": closed, "bbox": [lo, hi], "vertices": nv, "faces": nf, "meshes": len(meshes)}


def shown(doc, o, vp):
    """The meshes Rhino shows for an object: its custom render meshes, waiting for an
    asynchronous provider (displacement)."""
    F = rr.CustomRenderMeshes.RenderMeshProvider.Flags
    for attempt in range(200):
        res = o.RenderMeshes(rg.MeshType.Render, vp, None, getattr(F, "None"), None, None)
        rm, flags = res if isinstance(res, tuple) else (res, getattr(F, "None"))
        if (int(flags) & int(F.Incomplete)) == 0 or rm is None:
            break
        Rhino.RhinoApp.Wait()
        System.Threading.Thread.Sleep(50)
    else:
        note("%s: still incomplete" % o.Attributes.Name)
    out = []
    if rm is not None:
        for inst in rm:
            m = inst.Mesh.DuplicateMesh()
            m.Transform(inst.Transform)
            out.append(m)
    return out


def dump(meshes):
    return [{"v": [[q.X, q.Y, q.Z] for q in (m.Vertices.Point3dAt(i) for i in range(m.Vertices.Count))],
             "f": [[f.A, f.B, f.C, f.D] for f in (m.Faces[i] for i in range(m.Faces.Count))]} for m in meshes]


def main():
    progress("start")
    doc = sc.doc
    objs = build(doc)
    path = os.path.join(OUT, "modifiers.3dm")
    doc.WriteFile(path, Rhino.FileIO.FileWriteOptions())
    vp = Rhino.DocObjects.ViewportInfo(doc.Views.ActiveView.ActiveViewport)
    out = {"notes": NOTES, "objects": []}
    dumps = {}
    for name, oid in objs:
        progress("measure " + name)
        o = doc.Objects.FindId(oid)
        meshes = shown(doc, o, vp)
        plain = list(o.GetMeshes(rg.MeshType.Render) or [])
        r = {"name": name, "id": str(oid), "shown": stats(meshes), "plain": stats(plain) if plain else None}
        out["objects"].append(r)
        if DUMP:
            dumps[name] = {"shown": dump(meshes), "plain": dump(plain)}
    with open(os.path.join(OUT, "modifiers.json"), "w") as fh:
        json.dump(out, fh, indent=1, sort_keys=True)
    if DUMP:
        with open(os.path.join(OUT, "modifiers_dump.json"), "w") as fh:
            json.dump(dumps, fh)


try:
    main()
except Exception:
    with open(os.path.join(OUT, "modifiers_error.txt"), "w") as fh:
        fh.write(traceback.format_exc() + "\n" + "\n".join(NOTES))

sc.doc.Modified = False
