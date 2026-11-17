# -*- coding: utf-8 -*-
"""Materials (materials.3dm): a plain material with every colour set, one with a bitmap texture
(materials_texture.png, written beside it) and a bump, a physically based one, and a mesh with
texture coordinates that uses the textured material; with what Rhino reads of them
(materials.json)."""
import os
import json
import struct
import zlib
import Rhino
import Rhino.Geometry as rg
import Rhino.DocObjects as rd
import scriptcontext as sc
import System.Drawing as sd

OUT = os.environ.get("MODELS_OUT")


def png(path, w, h):
    """A small checker image, RGB."""
    rows = b""
    for y in range(h):
        rows += b"\x00"
        for x in range(w):
            rows += b"\xff\x80\x00" if (x // 4 + y // 4) % 2 else b"\x20\x40\xc0"

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    with open(path, "wb") as fh:
        fh.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))


def color(c):
    return [int(c.R), int(c.G), int(c.B), int(c.A)]


def color4f(c):
    return [float(c.R), float(c.G), float(c.B), float(c.A)]


def blend(t):
    """The blend constant of a texture (`m_blend_constant_A`)."""
    r = t.GetAlphaBlendValues()
    return r[1] if isinstance(r, tuple) else None


def main():
    doc = sc.doc
    for i in [o.Id for o in doc.Objects]:
        doc.Objects.Delete(i, True)
    image = os.path.join(OUT, "materials_texture.png")
    bump = os.path.join(OUT, "materials_bump.png")
    png(image, 16, 16)
    png(bump, 8, 8)

    plain = rd.Material()
    plain.Name = "Plain"
    plain.AmbientColor = sd.Color.FromArgb(10, 20, 30)
    plain.DiffuseColor = sd.Color.FromArgb(200, 100, 50)
    plain.EmissionColor = sd.Color.FromArgb(5, 6, 7)
    plain.SpecularColor = sd.Color.FromArgb(240, 230, 220)
    plain.ReflectionColor = sd.Color.FromArgb(90, 80, 70)
    plain.TransparentColor = sd.Color.FromArgb(11, 22, 33)
    plain.Shine = 0.4 * rd.Material.MaxShine
    plain.Reflectivity = 0.3
    plain.IndexOfRefraction = 1.45
    plain.Transparency = 0.25
    plain_index = doc.Materials.Add(plain)

    textured = rd.Material()
    textured.Name = "Textured"
    textured.DiffuseColor = sd.Color.FromArgb(255, 255, 255)
    textured.SetBitmapTexture(image)
    textured.SetBumpTexture(bump)
    textures = textured.GetTextures()
    for t in textures:
        if t.TextureType == rd.TextureType.Bitmap:
            t.UvwTransform = rg.Transform.Scale(rg.Point3d.Origin, 2.0)
            textured.SetTexture(t, rd.TextureType.Bitmap)
    textured_index = doc.Materials.Add(textured)

    pbr = rd.Material()
    pbr.Name = "Metal"
    pbr.ToPhysicallyBased()
    p = pbr.PhysicallyBased
    p.BaseColor = Rhino.Display.Color4f(0.8, 0.5, 0.25, 1.0)
    p.Metallic = 0.9
    p.Roughness = 0.35
    p.Opacity = 0.75
    p.Emission = Rhino.Display.Color4f(0.1, 0.2, 0.3, 1.0)
    p.Clearcoat = 0.2
    pbr_index = doc.Materials.Add(pbr)

    def add(geometry, name, index):
        a = rd.ObjectAttributes()
        a.Name = name
        a.MaterialIndex = index
        a.MaterialSource = rd.ObjectMaterialSource.MaterialFromObject
        if isinstance(geometry, rg.Mesh):
            return doc.Objects.AddMesh(geometry, a)
        return doc.Objects.AddBrep(geometry, a)

    mesh = rg.Mesh()
    for x, y in [(0, 0), (10, 0), (10, 10), (0, 10), (20, 0), (20, 10)]:
        mesh.Vertices.Add(x, y, 0)
    for u, v in [(0, 0), (0.5, 0), (0.5, 1), (0, 1), (1, 0), (1, 1)]:
        mesh.TextureCoordinates.Add(u, v)
    mesh.Faces.AddFace(0, 1, 2, 3)
    mesh.Faces.AddFace(1, 4, 5, 2)
    mesh.Normals.ComputeNormals()
    add(mesh, "Textured mesh", textured_index)
    add(rg.Box(rg.Plane.WorldXY, rg.Interval(30, 40), rg.Interval(0, 10), rg.Interval(0, 10)).ToBrep(), "Plain box", plain_index)
    add(rg.Sphere(rg.Point3d(55, 5, 5), 5).ToBrep(), "Metal sphere", pbr_index)

    doc.WriteFile(os.path.join(OUT, "materials.3dm"), Rhino.FileIO.FileWriteOptions())

    # what Rhino reads back
    f = Rhino.FileIO.File3dm.Read(os.path.join(OUT, "materials.3dm"))
    out = {"materials": [], "meshes": [], "bitmaps": []}
    for m in f.AllMaterials:
        r = {
            "index": m.Index,
            "name": m.Name,
            "ambient": color(m.AmbientColor),
            "diffuse": color(m.DiffuseColor),
            "emission": color(m.EmissionColor),
            "specular": color(m.SpecularColor),
            "reflection": color(m.ReflectionColor),
            "transparent": color(m.TransparentColor),
            "shine": m.Shine,
            "reflectivity": m.Reflectivity,
            "ior": m.IndexOfRefraction,
            "transparency": m.Transparency,
            "textures": [],
            "pbr": None,
        }
        for t in m.GetTextures():
            r["textures"].append({
                "file": t.FileReference.FullPath if t.FileReference is not None else t.FileName,
                "type": int(t.TextureType),
                "enabled": t.Enabled,
                "mode": int(t.TextureCombineMode),
                "blend": blend(t),
                "uvw": [t.UvwTransform[i, j] for i in range(4) for j in range(4)],
            })
        if m.IsPhysicallyBased:
            q = m.PhysicallyBased
            r["pbr"] = {
                "base_color": color4f(q.BaseColor),
                "metallic": q.Metallic,
                "roughness": q.Roughness,
                "specular": q.Specular,
                "opacity": q.Opacity,
                "opacity_ior": q.OpacityIOR,
                "clearcoat": q.Clearcoat,
                "emission": color4f(q.Emission),
                "alpha": q.Alpha,
            }
        out["materials"].append(r)
    for ob in f.Objects:
        g = ob.Geometry
        if isinstance(g, rg.Mesh):
            out["meshes"].append({
                "name": ob.Attributes.Name,
                "material": ob.Attributes.MaterialIndex,
                "texture_coordinates": [[float(t.X), float(t.Y)] for t in g.TextureCoordinates],
            })
    try:
        for e in f.EmbeddedFiles:
            out["bitmaps"].append({"file": e.Filename, "length": e.Length})
    except Exception:
        pass
    with open(os.path.join(OUT, "materials.json"), "w") as fh:
        json.dump(out, fh, indent=1)


try:
    main()
except Exception:
    import traceback
    with open(os.path.join(OUT, "materials_error.txt"), "w") as fh:
        fh.write(traceback.format_exc())
