# -*- coding: utf-8 -*-
"""Named views (views.3dm): a box and three views saved by name (a perspective from the
south-west, one from above, a parallel elevation), with the cameras Rhino reads back
(views.json)."""
import os
import json
import Rhino
import Rhino.Geometry as rg
import scriptcontext as sc

OUT = os.environ.get("MODELS_OUT")


def main():
    doc = sc.doc
    for i in [o.Id for o in doc.Objects]:
        doc.Objects.Delete(i, True)
    box = rg.Box(rg.Plane.WorldXY, rg.Interval(0, 10), rg.Interval(0, 6), rg.Interval(0, 3))
    doc.Objects.AddBrep(box.ToBrep())
    persp = [v for v in doc.Views if v.ActiveViewport.Name == "Perspective"][0]
    vp = persp.ActiveViewport
    vp.SetCameraLocations(rg.Point3d(5, 3, 1.5), rg.Point3d(-20, -25, 15))
    vp.Camera35mmLensLength = 35
    doc.NamedViews.Add("South-west", vp.Id)
    vp.SetCameraLocations(rg.Point3d(5, 3, 0), rg.Point3d(5, 3, 40))
    vp.CameraUp = rg.Vector3d.YAxis
    doc.NamedViews.Add("Above", vp.Id)
    front = [v for v in doc.Views if v.ActiveViewport.Name == "Front"][0]
    fp = front.ActiveViewport
    fp.ZoomBoundingBox(box.BoundingBox)
    doc.NamedViews.Add("Elevation", fp.Id)
    doc.WriteFile(os.path.join(OUT, "views.3dm"), Rhino.FileIO.FileWriteOptions())
    f = Rhino.FileIO.File3dm.Read(os.path.join(OUT, "views.3dm"))
    out = []
    for v in f.NamedViews:
        p = v.Viewport
        l, r, b, t, n, fa = p.GetFrustum()[1:] if isinstance(p.GetFrustum(), tuple) else (0, 0, 0, 0, 0, 0)
        out.append({
            "name": v.Name,
            "perspective": p.IsPerspectiveProjection,
            "location": [p.CameraLocation.X, p.CameraLocation.Y, p.CameraLocation.Z],
            "direction": [p.CameraDirection.X, p.CameraDirection.Y, p.CameraDirection.Z],
            "up": [p.CameraUp.X, p.CameraUp.Y, p.CameraUp.Z],
            "frustum": [l, r, b, t, n, fa],
        })
    with open(os.path.join(OUT, "views.json"), "w") as fh:
        json.dump(out, fh, indent=1)


try:
    main()
except Exception:
    import traceback
    with open(os.path.join(OUT, "views_error.txt"), "w") as fh:
        fh.write(traceback.format_exc())
