#! python3
# -*- coding: utf-8 -*-
"""Rendering data (render.3dm): render settings (size, colours, a custom environment for the
background and reflections, sun at a date and place, skylight, ground plane at an altitude,
linear workflow, dithering, safe frame; a post effect is tried, but RhinoCommon cannot switch
one on from a script), render content (a
Physically Based, a Plaster and a Custom material with a bitmap texture, a checker texture and
the environment), materials on an object and a layer, a decal on a box, displacement and edge
softening on a box, thickening on a surface, curve piping on a curve and shut lining on a
surface (without curves: the API has no way to add them to a document object). The mesh
modifiers are set as the parameters of Rhino's custom render mesh providers. With what Rhino 9
reads back of the saved file (render.json, File3dm)."""
import os
import json
import traceback
import xml.etree.ElementTree as ET
import System
import clr
import Rhino
import Rhino.Geometry as rg
import Rhino.Render as rr
import scriptcontext as sc

OUT = os.environ.get("MODELS_OUT")
NOTES = []


def note(what):
    NOTES.append(what)


def vec(v):
    return [v.X, v.Y, v.Z]


def color(c):
    return [int(c.R), int(c.G), int(c.B), int(c.A)]


def guid(g):
    return str(g)


def tryset(what, f):
    try:
        f()
    except Exception as e:
        note("%s: %s" % (what, e))


def plain(v):
    """A RhinoCommon value as JSON."""
    if v is None:
        return None
    if isinstance(v, (bool, int, float, str)):
        return v
    t = v.GetType().Name if hasattr(v, "GetType") else type(v).__name__
    if t in ("Color4f",):
        return [v.R, v.G, v.B, v.A]
    if t in ("Color",):
        return [int(v.R), int(v.G), int(v.B), int(v.A)]
    if t in ("Point3d", "Vector3d"):
        return [v.X, v.Y, v.Z]
    if t in ("Point2d", "Vector2d"):
        return [v.X, v.Y]
    if t in ("Point4d",):
        return [v.X, v.Y, v.Z, v.W]
    if t == "Guid":
        return str(v)
    if t in ("Double", "Single"):
        return float(v)
    if t in ("Int32", "Int64", "UInt32"):
        return int(v)
    if t == "Boolean":
        return bool(v)
    return str(v)


def build(doc):
    for i in [o.Id for o in doc.Objects]:
        doc.Objects.Delete(i, True)
    C = System.Drawing.Color.FromArgb
    U = rr.ContentUuids

    # render content: a checker texture, three materials, an environment
    checker = rr.RenderContent.Create(doc, U.Texture2DCheckerTextureType)
    checker.BeginChange(rr.RenderContent.ChangeContexts.Program)
    checker.Name = "Checker"
    checker.SetParameter("color-one", Rhino.Display.Color4f(0.9, 0.1, 0.1, 1.0))
    checker.SetParameter("color-two", Rhino.Display.Color4f(0.1, 0.1, 0.9, 1.0))
    checker.EndChange()
    if doc.RenderTextures.Find(checker.Id) is None:
        doc.RenderTextures.Add(checker)

    pbr = rr.RenderContent.Create(doc, U.PhysicallyBasedMaterialType)
    pbr.BeginChange(rr.RenderContent.ChangeContexts.Program)
    pbr.Name = "Brushed copper"
    pbr.Notes = "physically based"
    pbr.Tags = "metal;test"
    tryset("pbr base", lambda: pbr.SetParameter("pbr-base-color", Rhino.Display.Color4f(0.8, 0.45, 0.3, 1.0)))
    tryset("pbr metallic", lambda: pbr.SetParameter("pbr-metallic", 0.9))
    tryset("pbr roughness", lambda: pbr.SetParameter("pbr-roughness", 0.35))
    pbr.EndChange()
    if doc.RenderMaterials.Find(pbr.Id) is None:
        doc.RenderMaterials.Add(pbr)

    plaster = rr.RenderContent.Create(doc, U.PlasterMaterialType)
    plaster.BeginChange(rr.RenderContent.ChangeContexts.Program)
    plaster.Name = "Wall plaster"
    tryset("plaster colour", lambda: plaster.SetParameter("color", Rhino.Display.Color4f(0.95, 0.9, 0.8, 1.0)))
    plaster.EndChange()
    if doc.RenderMaterials.Find(plaster.Id) is None:
        doc.RenderMaterials.Add(plaster)

    custom = rr.RenderContent.Create(doc, U.BasicMaterialType)
    custom.BeginChange(rr.RenderContent.ChangeContexts.Program)
    custom.Name = "Tiles"
    tryset("custom diffuse", lambda: custom.SetParameter("diffuse", Rhino.Display.Color4f(0.2, 0.6, 0.3, 1.0)))
    tryset("custom shine", lambda: custom.SetParameter("shine", 0.4))
    custom.EndChange()
    if doc.RenderMaterials.Find(custom.Id) is None:
        doc.RenderMaterials.Add(custom)
    # a bitmap texture in the custom material's bitmap slot
    bitmap = os.path.join(OUT, "materials_texture.png")
    def add_tile():
        sim = rr.SimulatedTexture(doc)
        sim.Filename = bitmap
        tile = rr.RenderTexture.NewBitmapTexture(sim)
        tile.BeginChange(rr.RenderContent.ChangeContexts.Program)
        tile.Name = "Tile image"
        tile.EndChange()
        custom.BeginChange(rr.RenderContent.ChangeContexts.Program)
        custom.SetChild(tile, "bitmap-texture")
        custom.EndChange()
    tryset("bitmap child", add_tile)
    tryset("bitmap slot on", lambda: custom.SetChildSlotOn("bitmap-texture", True, rr.RenderContent.ChangeContexts.Program))
    tryset("bitmap slot amount", lambda: custom.SetChildSlotAmount("bitmap-texture", 80.0, rr.RenderContent.ChangeContexts.Program))

    env = rr.RenderContent.Create(doc, U.BasicEnvironmentType)
    env.BeginChange(rr.RenderContent.ChangeContexts.Program)
    env.Name = "Evening sky"
    tryset("env colour", lambda: env.SetParameter("background-color", Rhino.Display.Color4f(0.3, 0.4, 0.7, 1.0)))
    env.EndChange()
    if doc.RenderEnvironments.Find(env.Id) is None:
        doc.RenderEnvironments.Add(env)

    # objects
    box = rg.Box(rg.Plane.WorldXY, rg.Interval(0, 10), rg.Interval(0, 6), rg.Interval(0, 3)).ToBrep()
    a = Rhino.DocObjects.ObjectAttributes()
    a.Name = "Decal box"
    box_id = doc.Objects.AddBrep(box, a)
    ob = doc.Objects.FindId(box_id)
    ob.RenderMaterial = pbr
    ob.CommitChanges()

    layer = doc.Layers.Add("Plastered", C(200, 200, 180))
    tryset("layer material", lambda: setattr(doc.Layers[layer], "RenderMaterial", plaster))
    a = Rhino.DocObjects.ObjectAttributes()
    a.Name = "Displaced box"
    a.LayerIndex = layer
    disp_id = doc.Objects.AddBrep(rg.Box(rg.Plane.WorldXY, rg.Interval(20, 26), rg.Interval(0, 6), rg.Interval(0, 4)).ToBrep(), a)

    a = Rhino.DocObjects.ObjectAttributes()
    a.Name = "Thick surface"
    srf = rg.NurbsSurface.CreateFromCorners(rg.Point3d(0, 20, 0), rg.Point3d(10, 20, 0), rg.Point3d(10, 30, 2), rg.Point3d(0, 30, 2))
    thick_id = doc.Objects.AddSurface(srf, a)
    def tiles():
        o = doc.Objects.FindId(thick_id)
        o.RenderMaterial = custom
        o.CommitChanges()
    tryset("tiles on surface", tiles)

    a = Rhino.DocObjects.ObjectAttributes()
    a.Name = "Piped curve"
    crv = rg.Curve.CreateInterpolatedCurve([rg.Point3d(0, 40, 0), rg.Point3d(5, 44, 2), rg.Point3d(10, 40, 4), rg.Point3d(15, 45, 1)], 3)
    pipe_id = doc.Objects.AddCurve(crv, a)

    a = Rhino.DocObjects.ObjectAttributes()
    a.Name = "Shut lined surface"
    cyl = rg.Cylinder(rg.Circle(rg.Plane(rg.Point3d(30, 30, 0), rg.Vector3d.ZAxis), 3.0), 8.0).ToNurbsSurface()
    shut_id = doc.Objects.AddSurface(cyl, a)

    # a decal on the box
    def add_decal():
        p = rr.DecalCreateParams()
        p.TextureInstanceId = checker.Id
        p.DecalMapping = rr.DecalMapping.Planar
        p.DecalProjection = rr.DecalProjection.Forward
        p.MapToInside = False
        p.Transparency = 0.25
        p.Origin = rg.Point3d(2, 0, 0.5)
        p.VectorUp = rg.Vector3d(0, 0, 2)
        p.VectorAcross = rg.Vector3d(4, 0, 0)
        p.Height = 2.0
        p.Radius = 1.5
        p.StartLatitude = -0.5
        p.EndLatitude = 0.5
        p.StartLongitude = 0.0
        p.EndLongitude = 1.0
        p.MinU = 0.0
        p.MinV = 0.1
        p.MaxU = 0.9
        p.MaxV = 1.0
        d = rr.Decal.Create(p)
        o = doc.Objects.FindId(box_id)
        o.Attributes.Decals.Add(d)
        o.CommitChanges()
        if len(list(doc.Objects.FindId(box_id).Attributes.Decals)) == 0:
            note("decal: not on the object after CommitChanges, trying ModifyAttributes")
            attrs = o.Attributes.Duplicate()
            attrs.Decals.Add(d)
            doc.Objects.ModifyAttributes(box_id, attrs, True)
    tryset("decal", add_decal)

    # mesh modifiers: the custom render mesh parameters of their providers (the ids of their
    # openNURBS user data), named as in their XML
    ids = Rhino.Render.CustomRenderMeshes.MeshProviderIds
    DISPLACEMENT = ids.Displacement
    EDGE_SOFTENING = ids.EdgeSoftening
    THICKENING = ids.Thickening
    CURVE_PIPING = ids.CurvePiping
    SHUT_LINING = ids.Shutlining
    note("provider ids: %s %s %s %s %s" % (DISPLACEMENT, EDGE_SOFTENING, THICKENING, CURVE_PIPING, SHUT_LINING))

    def modify(oid, provider, values):
        o = doc.Objects.FindId(oid)
        for k, v in values:
            try:
                o.SetCustomRenderMeshParameter(provider, k, v)
            except Exception as e:
                note("modifier %s %s: %s" % (provider, k, e))
        o.CommitChanges()

    modify(disp_id, DISPLACEMENT, [("on", True), ("texture", checker.Id), ("channel", 2), ("black-point", 0.1), ("white-point", 0.8),
                                   ("sweep-pitch", 900), ("refine-steps", 2), ("refine-sensitivity", 0.4), ("post-weld-angle", 35.0),
                                   ("fairing-enabled", True), ("fairing-amount", 3), ("face-count-limit-enabled", True),
                                   ("face-count-limit", 20000), ("mesh-memory-limit", 256)])
    modify(disp_id, EDGE_SOFTENING, [("on", True), ("softening", 0.3), ("chamfer", True), ("unweld", True), ("edge-threshold", 7.5), ("force-softening", True)])
    modify(thick_id, THICKENING, [("on", True), ("distance", 0.4), ("solid", True), ("offset-only", False), ("both-sides", True)])
    modify(pipe_id, CURVE_PIPING, [("on", True), ("radius", 0.35), ("segments", 12), ("weld", False), ("accuracy", 70), ("cap-type", "box")])
    modify(shut_id, SHUT_LINING, [("on", True), ("faceted", True), ("auto-update", True), ("force-update", False)])
    for oid, provider in ((disp_id, DISPLACEMENT), (thick_id, THICKENING)):
        try:
            note("readback %s on: %s" % (provider, doc.Objects.FindId(oid).GetCustomRenderMeshParameter(provider, "on")))
        except Exception as e:
            note("readback %s: %s" % (provider, e))

    # render settings
    rs = doc.RenderSettings
    rs.UseViewportSize = False
    rs.ImageSize = System.Drawing.Size(1600, 900)
    rs.ImageDpi = 150.0
    rs.AmbientLight = C(30, 40, 50)
    rs.BackgroundColorTop = C(10, 20, 200)
    rs.BackgroundColorBottom = C(240, 200, 100)
    rs.BackgroundStyle = Rhino.Display.BackgroundStyle.Environment
    rs.RenderBackfaces = False
    rs.RenderCurves = True
    rs.TransparentBackground = False
    rs.UseHiddenLights = True
    tryset("antialias", lambda: setattr(rs, "AntialiasLevel", Rhino.Render.AntialiasLevel.High))
    tryset("background env", lambda: rs.SetRenderEnvironmentId(rr.RenderSettings.EnvironmentUsage.Background, env.Id))
    tryset("reflection env", lambda: rs.SetRenderEnvironmentId(rr.RenderSettings.EnvironmentUsage.Reflection, env.Id))
    tryset("reflection override", lambda: rs.SetRenderEnvironmentOverride(rr.RenderSettings.EnvironmentUsage.Reflection, True))

    sun = rs.Sun
    sun.BeginChange(rr.RenderContent.ChangeContexts.Program)
    sun.Enabled = True
    sun.ManualControlOn = False
    sun.North = 80.0
    sun.Latitude = 52.01
    sun.Longitude = 4.36
    sun.TimeZone = 1.0
    sun.DaylightSavingOn = True
    sun.DaylightSavingMinutes = 60
    sun.SetDateTime(System.DateTime(2026, 6, 21, 15, 30, 0), System.DateTimeKind.Local)
    sun.Intensity = 1.5
    sun.EndChange()

    sky = rs.Skylight
    sky.BeginChange(rr.RenderContent.ChangeContexts.Program)
    sky.Enabled = True
    sky.ShadowIntensity = 0.7
    sky.EndChange()

    lw = rs.LinearWorkflow
    lw.BeginChange(rr.RenderContent.ChangeContexts.Program)
    tryset("lw pre", lambda: setattr(lw, "PreProcessColors", True))
    tryset("lw pre tex", lambda: setattr(lw, "PreProcessTextures", True))
    tryset("lw pre gamma", lambda: setattr(lw, "PreProcessGamma", 2.2))
    tryset("lw post on", lambda: setattr(lw, "PostProcessGammaOn", True))
    tryset("lw post gamma", lambda: setattr(lw, "PostProcessGamma", 2.4))
    lw.EndChange()

    di = rs.Dithering
    di.BeginChange(rr.RenderContent.ChangeContexts.Program)
    di.Enabled = True
    di.Method = rr.Dithering.Methods.SimpleNoise
    di.EndChange()

    sf = rs.SafeFrame
    sf.BeginChange(rr.RenderContent.ChangeContexts.Program)
    sf.Enabled = True
    sf.PerspectiveOnly = False
    sf.FieldsOn = True
    sf.LiveFrameOn = True
    sf.ActionFrameOn = True
    sf.ActionFrameLinked = False
    sf.ActionFrameXScale = 0.85
    sf.ActionFrameYScale = 0.8
    sf.TitleFrameOn = False
    sf.TitleFrameXScale = 0.75
    sf.EndChange()

    gp = rs.GroundPlane
    gp.BeginChange(rr.RenderContent.ChangeContexts.Program)
    gp.Enabled = True
    gp.Altitude = -1.25
    gp.AutoAltitude = False
    gp.ShadowOnly = False
    gp.ShowUnderside = True
    gp.MaterialInstanceId = plaster.Id
    tryset("gp offset", lambda: setattr(gp, "TextureOffset", rg.Vector2d(0.5, 0.25)))
    tryset("gp size", lambda: setattr(gp, "TextureSize", rg.Vector2d(2.0, 3.0)))
    tryset("gp rotation", lambda: setattr(gp, "TextureRotation", 15.0))
    gp.EndChange()

    def post_effect():
        pe = rs.PostEffects
        pe.BeginChange(rr.RenderContent.ChangeContexts.Program)
        known = []
        for t in (rr.PostEffects.PostEffectType.Early, rr.PostEffects.PostEffectType.ToneMapping, rr.PostEffects.PostEffectType.Late):
            for p in doc.PostEffects.GetPostEffects(t):
                known.append(p)
        note("post effects: %s" % [(str(p.Id), p.LocalName) for p in known])
        for p in known:
            if p.LocalName in ("Glare", "Bloom", "Glow", "Fog"):
                d = pe.PostEffectDataFromId(p.Id)
                if d is None:
                    note("post effect %s: no data" % p.LocalName)
                    continue
                d.On = True
                d.Shown = True
                note("post effect %s on" % p.LocalName)
                break
        pe.EndChange()
    tryset("post effect", post_effect)

    doc.RenderSettings = rs
    # also through the document's own ground plane
    tryset("doc ground plane", lambda: (setattr(doc.GroundPlane, "Enabled", True), setattr(doc.GroundPlane, "Altitude", -1.25), setattr(doc.GroundPlane, "AutoAltitude", False)))
    return {"box": guid(box_id), "displaced": guid(disp_id), "thick": guid(thick_id), "piped": guid(pipe_id), "shut": guid(shut_id)}


def variant(v):
    """A Rhino.Render.Variant (or plain value) as its type and value."""
    if v is None:
        return None
    if not hasattr(v, "ToGuid"):
        return {"type": type(v).__name__, "value": plain(v)}
    t = str(v.Type)
    try:
        if t == "Bool":
            x = bool(v.ToBoolean(None))
        elif t == "Integer":
            x = int(v.ToInt32(None))
        elif t in ("Float", "Double"):
            x = float(v.ToDouble(None))
        elif t == "Color":
            c = v.ToColor4f()
            x = [c.R, c.G, c.B, c.A]
        elif t == "Vector2d":
            q = v.ToVector2d()
            x = [q.X, q.Y]
        elif t == "Vector3d":
            q = v.ToVector3d()
            x = [q.X, q.Y, q.Z]
        elif t == "Point4d":
            q = v.ToPoint4d()
            x = [q.X, q.Y, q.Z, q.W]
        elif t == "Uuid":
            x = str(v.ToGuid())
        elif t == "Matrix":
            m = v.ToTransform()
            x = [m[i, j] for i in range(4) for j in range(4)]
        else:
            x = str(v)
    except Exception as e:
        x = "error: %s" % e
    return {"type": t, "value": x}


def content_record(c, doc_content):
    r = {
        "kind": str(c.Kind),
        "type_name": c.TypeName,
        "type_id": guid(c.TypeId),
        "name": c.Name,
        "id": guid(c.Id),
        "render_engine_id": guid(c.RenderEngineId),
        "plug_in_id": guid(c.PlugInId),
        "group_id": guid(c.GroupId),
        "notes": c.Notes,
        "tags": c.Tags,
        "hidden": c.Hidden,
        "reference": c.Reference,
        "auto_delete": c.AutoDelete,
        "child_slot_name": c.ChildSlotName,
        "parent": guid(c.Parent.Id) if c.Parent is not None else None,
        "children": [guid(k.Id) for k in c.Children],
        "parameters": {},
    }
    try:
        if c.Kind.ToString() == "Texture" or hasattr(c, "Filename"):
            r["filename"] = c.Filename
    except Exception:
        pass
    # the parameters by the names of the document content's XML
    names = []
    if doc_content is not None:
        try:
            root = ET.fromstring(doc_content.Xml)
            for tag in ("parameters-v8", "parameters"):
                p = root.find(tag)
                if p is not None:
                    names = [(e.tag if tag == "parameters" else e.get("name")) for e in p]
                    r["doc_xml_parameters_tag"] = tag
                    break
        except Exception as e:
            note("xml %s: %s" % (c.Name, e))
    for n in names:
        if not n:
            continue
        try:
            r["parameters"][n] = variant(c.GetParameter(n))
        except Exception as e:
            r["parameters"][n] = "error: %s" % e
    return r


def main():
    doc = sc.doc
    ids = build(doc)
    path = os.path.join(OUT, "render.3dm")
    doc.WriteFile(path, Rhino.FileIO.FileWriteOptions())
    f = Rhino.FileIO.File3dm.Read(path)
    out = {"ids": ids, "notes": NOTES}

    rs = f.Settings.RenderSettings
    out["render_settings"] = {
        "use_viewport_size": rs.UseViewportSize,
        "image_size": [rs.ImageSize.Width, rs.ImageSize.Height],
        "image_dpi": rs.ImageDpi,
        "image_unit_system": int(rs.ImageUnitSystem),
        "ambient_light": color(rs.AmbientLight),
        "background_style": int(rs.BackgroundStyle),
        "background_color_top": color(rs.BackgroundColorTop),
        "background_color_bottom": color(rs.BackgroundColorBottom),
        "use_hidden_lights": rs.UseHiddenLights,
        "depth_cue": rs.DepthCue,
        "flat_shade": rs.FlatShade,
        "render_backfaces": rs.RenderBackfaces,
        "render_points": rs.RenderPoints,
        "render_curves": rs.RenderCurves,
        "render_isoparams": rs.RenderIsoparams,
        "render_mesh_edges": rs.RenderMeshEdges,
        "render_annotations": rs.RenderAnnotations,
        "scale_background_to_fit": rs.ScaleBackgroundToFit,
        "transparent_background": rs.TransparentBackground,
        "antialias_level": int(rs.AntialiasLevel),
        "shadowmap_level": int(rs.ShadowmapLevel),
        "render_source": int(rs.RenderSource),
        "named_view": rs.NamedView,
        "snapshot": rs.Snapshot,
        "specific_viewport": rs.SpecificViewport,
    }
    U = rr.RenderSettings.EnvironmentUsage
    out["environments"] = {}
    for name, usage in (("background", U.Background), ("reflection", U.Reflection), ("skylighting", U.Skylighting)):
        e = {}
        tryset("env id " + name, lambda: e.__setitem__("id", guid(rs.RenderEnvironmentId(usage, rr.RenderSettings.EnvironmentPurpose.Standard))))
        tryset("env for rendering " + name, lambda: e.__setitem__("for_rendering", guid(rs.RenderEnvironmentId(usage, rr.RenderSettings.EnvironmentPurpose.ForRendering))))
        if name != "background":
            tryset("env override " + name, lambda: e.__setitem__("override", rs.RenderEnvironmentOverride(usage)))
        out["environments"][name] = e

    gp = rs.GroundPlane
    out["ground_plane"] = {
        "enabled": gp.Enabled, "altitude": gp.Altitude, "auto_altitude": gp.AutoAltitude,
        "shadow_only": gp.ShadowOnly, "show_underside": gp.ShowUnderside,
        "material": guid(gp.MaterialInstanceId), "texture_offset": plain(gp.TextureOffset),
        "texture_size": plain(gp.TextureSize), "texture_rotation": gp.TextureRotation,
        "texture_offset_locked": gp.TextureOffsetLocked, "texture_size_locked": gp.TextureSizeLocked,
    }
    s = rs.Sun
    when = s.GetDateTime(System.DateTimeKind.Local)
    out["sun"] = {
        "enabled": s.Enabled, "manual_control": s.ManualControlOn, "north": s.North,
        "azimuth": s.Azimuth, "altitude": s.Altitude, "latitude": s.Latitude, "longitude": s.Longitude,
        "time_zone": s.TimeZone, "daylight_saving_on": s.DaylightSavingOn,
        "daylight_saving_minutes": s.DaylightSavingMinutes, "intensity": s.Intensity,
        "skylight_on": s.SkylightOn, "accuracy": int(s.Accuracy),
        "local": [when.Year, when.Month, when.Day, when.Hour, when.Minute, when.Second],
        "vector": vec(s.Vector), "light_diffuse": color(s.Light.Diffuse),
        "light_direction": vec(s.Light.Direction), "light_intensity": s.Light.Intensity,
        "color_from_altitude": color(rr.Sun.ColorFromAltitude(s.Altitude)),
    }
    k = rs.Skylight
    out["skylight"] = {"enabled": k.Enabled, "shadow_intensity": k.ShadowIntensity,
                       "custom_environment_on": k.CustomEnvironmentOn, "custom_environment": guid(k.CustomEnvironment)}
    lw = rs.LinearWorkflow
    out["linear_workflow"] = {"pre_process_colors": lw.PreProcessColors, "pre_process_textures": lw.PreProcessTextures,
                              "pre_process_gamma": lw.PreProcessGamma, "post_process_gamma_on": lw.PostProcessGammaOn,
                              "post_process_gamma": lw.PostProcessGamma}
    d = rs.Dithering
    out["dithering"] = {"enabled": d.Enabled, "method": int(d.Method), "method_name": str(d.Method)}
    sf = rs.SafeFrame
    out["safe_frame"] = {"enabled": sf.Enabled, "perspective_only": sf.PerspectiveOnly, "fields_on": sf.FieldsOn,
                         "live_frame_on": sf.LiveFrameOn, "action_frame_on": sf.ActionFrameOn,
                         "action_frame_linked": sf.ActionFrameLinked, "action_frame_x_scale": sf.ActionFrameXScale,
                         "action_frame_y_scale": sf.ActionFrameYScale, "title_frame_on": sf.TitleFrameOn,
                         "title_frame_linked": sf.TitleFrameLinked, "title_frame_x_scale": sf.TitleFrameXScale,
                         "title_frame_y_scale": sf.TitleFrameYScale}
    out["post_effects"] = []
    try:
        for p in rs.PostEffects:
            out["post_effects"].append({"id": guid(p.Id), "name": p.LocalName, "on": p.On, "shown": p.Shown, "type": str(p.Type)})
    except Exception as e:
        note("post effects: %s" % e)
    out["doc_post_effects"] = []
    try:
        for p in sc.doc.RenderSettings.PostEffects:
            out["doc_post_effects"].append({"id": guid(p.Id), "name": p.LocalName, "on": p.On, "shown": p.Shown, "type": str(p.Type)})
    except Exception as e:
        note("doc post effects: %s" % e)
    try:
        rc = rs.RenderChannels
        out["render_channels"] = {"mode": int(rc.Mode), "custom": [guid(g) for g in rc.CustomList]}
    except Exception as e:
        note("render channels: %s" % e)

    # render content of the file, the parameters by the names the document's content has
    out["render_content"] = []
    doc = sc.doc
    for table, doctable in ((f.RenderMaterials, doc.RenderMaterials), (f.RenderEnvironments, doc.RenderEnvironments), (f.RenderTextures, doc.RenderTextures)):
        def walk(c):
            dc = rr.RenderContent.FromId(doc, c.Id)
            out["render_content"].append(content_record(c, dc))
            for k in c.Children:
                walk(k)
        for c in table:
            walk(c)

    out["materials"] = []
    for m in f.AllMaterials:
        r = {"name": m.Name, "index": m.Index, "id": guid(m.Id)}
        tryset("material rdk id", lambda: r.__setitem__("render_material", guid(m.RenderMaterialInstanceId)))
        out["materials"].append(r)
    out["layers"] = []
    for l in f.AllLayers:
        out["layers"].append({"name": l.Name, "index": l.Index, "material": l.RenderMaterialIndex})

    out["objects"] = []
    for o in f.Objects:
        at = o.Attributes
        r = {"id": guid(at.ObjectId), "name": at.Name, "material": at.MaterialIndex,
             "material_source": int(at.MaterialSource), "decals": []}
        try:
            for dc in at.Decals:
                b = dc.UVBounds(0.0, 0.0, 0.0, 0.0)
                if isinstance(b, tuple):
                    b = [x for x in b if x is not None]
                hs = None
                vs = None
                r["decals"].append({
                    "texture": guid(dc.TextureInstanceId), "mapping": int(dc.Mapping), "mapping_name": str(dc.Mapping),
                    "projection": int(dc.Projection), "projection_name": str(dc.Projection),
                    "map_to_inside": dc.MapToInside, "transparency": dc.Transparency, "visible": dc.IsVisible,
                    "origin": vec(dc.Origin), "vector_up": vec(dc.VectorUp), "vector_across": vec(dc.VectorAcross),
                    "height": dc.Height, "radius": dc.Radius,
                    "horz_sweep": [hs[0], hs[1]] if isinstance(hs, tuple) else [dc.StartLatitude, dc.EndLatitude],
                    "vert_sweep": [vs[0], vs[1]] if isinstance(vs, tuple) else [dc.StartLongitude, dc.EndLongitude],
                    "uv_bounds": list(b) if isinstance(b, list) else None,
                })
        except Exception as e:
            note("decals %s: %s" % (at.Name, e))
        mm = at.File3dmMeshModifiers
        mods = {}
        if mm is not None:
            x = mm.Displacement
            if x is not None:
                mods["displacement"] = {"on": x.On, "texture": guid(x.TextureId), "channel": x.MappingChannel,
                                        "black_point": x.BlackPoint, "white_point": x.WhitePoint,
                                        "sweep_pitch": x.InitialQuality, "refine_steps": x.RefineSteps,
                                        "refine_sensitivity": x.RefineSensitivity, "post_weld_angle": x.PostWeldAngle,
                                        "fairing_on": x.FairingOn, "fairing": x.Fairing,
                                        "face_limit_on": x.FinalMaxFacesOn, "face_limit": x.FinalMaxFaces,
                                        "memory_limit": x.MeshMemoryLimit, "sweep_formula": int(x.SweepResolutionFormula)}
            x = mm.EdgeSoftening
            if x is not None:
                mods["edge_softening"] = {"on": x.On, "softening": x.Softening, "chamfer": x.Chamfer, "faceted": x.Faceted,
                                          "edge_angle_threshold": x.EdgeAngleThreshold, "force_softening": x.ForceSoftening}
            x = mm.Thickening
            if x is not None:
                mods["thickening"] = {"on": x.On, "distance": x.Distance, "solid": x.Solid, "offset_only": x.OffsetOnly, "both_sides": x.BothSides}
            x = mm.CurvePiping
            if x is not None:
                mods["curve_piping"] = {"on": x.On, "radius": x.Radius, "segments": x.Segments, "faceted": x.Faceted,
                                        "accuracy": x.Accuracy, "cap_type": int(x.CapType), "cap_name": str(x.CapType)}
            x = mm.ShutLining
            if x is not None:
                curves = []
                for g in x.GetCurves():
                    curves.append({"id": guid(g), "enabled": x.CurveEnabled(g), "radius": x.CurveRadius(g), "profile": x.CurveProfile(g),
                                   "pull": x.CurvePull(g), "is_bump": x.CurveIsBump(g)})
                mods["shut_lining"] = {"on": x.On, "faceted": x.Faceted, "auto_update": x.AutoUpdate, "force_update": x.ForceUpdate, "curves": curves}
        r["mesh_modifiers"] = mods
        out["objects"].append(r)

    with open(os.path.join(OUT, "render.json"), "w") as fh:
        json.dump(out, fh, indent=1, sort_keys=True)


try:
    main()
except Exception:
    with open(os.path.join(OUT, "render_error.txt"), "w") as fh:
        fh.write(traceback.format_exc() + "\n" + "\n".join(NOTES))

# let Rhino exit without asking to save
sc.doc.Modified = False
