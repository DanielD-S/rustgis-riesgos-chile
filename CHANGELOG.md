# Changelog

## 0.1.1 (2026-11-06)

- The layers are styled in the RustGIS window too. 0.1.0 added them with `layer.addUrl`, which
  the desktop app downloads after the command has ended, so they kept RustGIS's own style there
  (only the command line waited for them). Now the plugin reads each layer's GeoJSON with
  `host.fetch` and adds it with `host.addLayer`, then styles it, in the same click. The answer
  goes from one to the other as it comes, without being read into the plugin, so the largest
  layers (1.8 MB) fit in a command. A layer `host.fetch` can't read (more than 2 MB, or an
  older RustGIS) is added with `layer.addUrl` as before.
- A layer added this way is kept in the project, as it was when added.

## 0.1.0 (2026-11-05)

- First version: 18 layers in the **Riesgos Chile** tab.
  - **Earthquakes** (USGS): M2.5+ of the last 24 hours, 7 days and 30 days, M4.5+ of the last 12
    months, sized by magnitude.
  - **Tsunami** (SENAPRED): areas to evacuate, evacuation routes, meeting points, safe line, 30 m
    contour 2026.
  - **Volcanoes** (SENAPRED): active volcanoes by hazard category, volcanic hazard areas,
    evacuation areas, routes and meeting points.
  - **Wildfires**: VIIRS hotspots inside Chile of the last 24 hours, 48 hours and 7 days (NASA
    FIRMS), wildfire recurrence 2020–2024 (SENAPRED).
- A weekly services check opens an issue when a layer's service stops answering.
