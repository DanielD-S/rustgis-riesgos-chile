# Riesgos Chile

Chile's natural hazards as one-click layers for RustGIS: recent earthquakes from the
[USGS](https://earthquake.usgs.gov), wildfire hotspots of the last days from
[NASA FIRMS](https://firms.modaps.eosdis.nasa.gov) (VIIRS), and the tsunami, volcanic and wildfire
hazard maps of [SENAPRED](https://www.senapred.cl) (the Servicio Nacional de Prevención y
Respuesta ante Desastres), as its [Visor Chile Preparado](https://www.visorchilepreparado.cl)
publishes them.

18 layers in a ribbon tab of their own, each a command that scripts, the command line and the AI
Assistant can run too. Each is styled when it lands: earthquakes sized by magnitude, volcanoes and
hazard areas coloured by category.

> The layers are for information and planning. In an emergency, follow SENAPRED's instructions.

## Its tab

**Riesgos Chile**, at the end of the ribbon's tab strip:

- **Earthquakes ▾**: of magnitude 2.5 or more in the last 24 hours, 7 days or 30 days, or of 4.5
  or more in the last 12 months, in and off Chile (the USGS's catalogue; the box from 17° to 57° S
  and 65° to 80° W takes in the trench and the deep earthquakes under Argentina and Bolivia)
- **Tsunami ▾**: areas to evacuate, evacuation routes, meeting points, the safe line, and the
  2026 30 m contour by commune (SENAPRED)
- **Volcanoes ▾**: the geologically active volcanoes by hazard category (from SERNAGEOMIN's
  ranking), volcanic hazard areas (high, medium, low), evacuation areas, routes and meeting points
  (SENAPRED)
- **Wildfires ▾**: hotspots of the last 24 hours, 48 hours or 7 days inside Chile (NASA FIRMS,
  VIIRS 375 m, through Esri's Living Atlas), and wildfire recurrence 2020–2024 (SENAPRED)
- **Help**: About Riesgos Chile

The large layers are read in the map view: tsunami evacuation routes and volcanic hazard areas
from 1:2,000,000 in, wildfire recurrence from 1:500,000 in. The others are read whole.

## Sources

| Layers | Source | Service |
|---|---|---|
| Earthquakes | USGS Earthquake Hazards Program | [FDSN event service](https://earthquake.usgs.gov/fdsnws/event/1/) |
| Hotspots | NASA FIRMS (VIIRS) | [Esri Living Atlas](https://services9.arcgis.com/RHVPKKiFTONKtxq3/arcgis/rest/services/Satellite_VIIRS_Thermal_Hotspots_and_Fire_Activity/FeatureServer/0) |
| Tsunami, volcanoes, wildfire recurrence | SENAPRED | [ArcGIS Online](https://services5.arcgis.com/i7S5PSnIJAUcWvSE/ArcGIS/rest/services) |

The hotspots are asked for inside Chile's outline (`src/outline.rs`, written by
`scripts/outline.py` from the INE's Census 2024 regions): it is widened by about 20 km and
simplified to 60 vertices to fit in a request's address, so it takes in a strip of Argentina along
the Andes.

Not yet: the Centro Sismológico Nacional publishes its catalogue only as web pages, which a plugin
can't read, and SERNAGEOMIN's live volcanic alert levels have no public service.

## Install

In RustGIS, **Plugins › Browse** lists Riesgos Chile once it is in the
[registry](https://github.com/opengeos/rustgis-plugins). It needs RustGIS 0.5.0 or later and the
`network` and `edit` permissions.

## Build

```bash
rustup target add wasm32-unknown-unknown
python scripts/package.py      # dist/riesgos-chile-<version>.rgplugin
cargo test
```

On Windows the tests need `RUSTFLAGS="-C link-arg=/FORCE:UNRESOLVED"`: the library names
RustGIS's host imports, which a native link can't resolve.

`python scripts/check_services.py` asks every layer's service for one feature, as the plugin
asks it; the **Services** workflow runs it every Monday and opens an issue when a service stops
answering. A tag `vX.Y.Z` publishes a release with the package and its registry entry.

## License

MIT. The data belong to their sources: see each layer's credit.
