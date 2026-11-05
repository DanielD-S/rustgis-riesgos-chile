"""Writes src/outline.rs: Chile's outline as an ArcGIS polygon, for the hotspot query.

    python scripts/outline.py

The INE's Census 2024 regions, joined, widened by about 20 km (so a hotspot on the coast or the
border still falls inside) and simplified to about 60 vertices at a tenth of a degree: the
filter goes in the request's address, and ArcGIS Online refuses one longer than 2,048
characters. Islands far from the continent (Rapa Nui, Juan Fernández, Desventuradas) are left
out. It reaches some 20 km into Argentina along the Andes.
"""

import json
import pathlib
import urllib.request

from shapely.geometry import box, mapping, shape
from shapely.ops import unary_union

ROOT = pathlib.Path(__file__).resolve().parent.parent
REGIONS = (
    "https://services5.arcgis.com/hUyD8u3TeZLKPe4T/arcgis/rest/services/L%C3%ADmites_DPA___Censo_2024/FeatureServer/2/query"
    "?where=1%3D1&outFields=&returnGeometry=true&outSR=4326&f=geojson&maxAllowableOffset=0.05"
)
WIDEN = 0.2  # degrees
SIMPLIFY = 0.3
CONTINENT = box(-76.5, -60, -60, -15)


def main() -> None:
    with urllib.request.urlopen(REGIONS, timeout=120) as r:
        fc = json.load(r)
    land = unary_union([shape(f["geometry"]).buffer(0).simplify(0.05) for f in fc["features"] if f["geometry"]])
    land = land.intersection(CONTINENT)
    outline = land.buffer(WIDEN).simplify(SIMPLIFY)
    parts = [p for p in getattr(outline, "geoms", [outline]) if p.area > 0.5]
    rings = [[[round(x, 1), round(y, 1)] for x, y in mapping(p)["coordinates"][0]] for p in parts]
    text = json.dumps({"rings": rings}, separators=(",", ":"))
    n = sum(len(r) for r in rings)
    missed = land.difference(unary_union([shape({"type": "Polygon", "coordinates": [r]}) for r in rings])).area
    rs = (
        "//! Chile's outline as an ArcGIS polygon (longitude/latitude), for the hotspot query: the INE's\n"
        "//! Census 2024 regions joined, widened by about 20 km and simplified to fit in a request's\n"
        f"//! address. Written by `scripts/outline.py`: {n} vertices.\n\n"
        f'pub const CHILE: &str = r#"{text}"#;\n'
    )
    (ROOT / "src").mkdir(exist_ok=True)
    (ROOT / "src/outline.rs").write_text(rs, encoding="utf-8")
    print(f"{len(rings)} parts, {n} vertices, {len(text)} characters; land left out: {missed:.4f} square degrees")


if __name__ == "__main__":
    main()
