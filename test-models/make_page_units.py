# -*- coding: utf-8 -*-
"""Page units other than the model's (page_units.3dm): a model in metres with its layouts in
inches, a layout with a line on it; what Rhino reads back in page_units.json."""
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
    doc.AdjustModelUnitSystem(Rhino.UnitSystem.Meters, False)
    doc.AdjustPageUnitSystem(Rhino.UnitSystem.Inches, False)
    doc.Objects.AddLine(rg.Line(rg.Point3d(0, 0, 0), rg.Point3d(2, 0, 0)))
    page = doc.Views.AddPageView("Sheet", 11.0, 8.5)
    a = Rhino.DocObjects.ObjectAttributes()
    a.Space = Rhino.DocObjects.ActiveSpace.PageSpace
    a.ViewportId = page.MainViewport.Id
    a.Name = "On the sheet"
    doc.Objects.AddLine(rg.Line(rg.Point3d(1, 1, 0), rg.Point3d(10, 1, 0)), a)
    doc.WriteFile(os.path.join(OUT, "page_units.3dm"), Rhino.FileIO.FileWriteOptions())
    f = Rhino.FileIO.File3dm.Read(os.path.join(OUT, "page_units.3dm"))
    out = {"model": str(f.Settings.ModelUnitSystem), "page": str(f.Settings.PageUnitSystem)}
    with open(os.path.join(OUT, "page_units.json"), "w") as fh:
        json.dump(out, fh, indent=1)


try:
    main()
except Exception:
    import traceback
    with open(os.path.join(OUT, "page_units_error.txt"), "w") as fh:
        fh.write(traceback.format_exc())
