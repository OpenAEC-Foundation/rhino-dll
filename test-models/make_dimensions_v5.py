# -*- coding: utf-8 -*-
"""Saves the open document as a Rhino 5 file (MODELS_OUT/SHOT_NAME.3dm)."""
import os
import Rhino
import scriptcontext as sc

OUT = os.environ.get("MODELS_OUT")
NAME = os.environ.get("SHOT_NAME", "v5")
o = Rhino.FileIO.FileWriteOptions()
o.FileVersion = 5
o.SuppressDialogBoxes = True
ok = sc.doc.WriteFile(os.path.join(OUT, NAME + ".3dm"), o)
with open(os.path.join(OUT, NAME + "_saved.txt"), "w") as f:
    f.write(str(ok))
sc.doc.Modified = False
