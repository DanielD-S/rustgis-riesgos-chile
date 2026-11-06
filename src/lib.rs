//! Riesgos Chile: Chile's natural hazards as one-click layers.
//!
//! Each layer in [`CATALOG`] is a command that reads its features as GeoJSON with `host.fetch`
//! (which needs the `network` permission), adds them and styles them (see `fetch.rs`):
//!
//! - Earthquakes: the USGS's catalogue (FDSN event service), of a period and a magnitude.
//! - Hotspots: NASA FIRMS's VIIRS detections, through Esri's Living Atlas layer, which can be
//!   asked for the hotspots inside Chile's outline (`outline.rs`).
//! - Tsunami, volcanic and wildfire hazard: SENAPRED's layers on ArcGIS Online. Small ones are
//!   read whole; the large ones in the map view, from a scale of their own in.

// On a native target (the tests) the entry points aren't exported, so the commands look unused.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use rustgis_plugin::{Request, Value, host, json};

mod fetch;
mod outline;

use fetch::Fetched;

/// `plugin.json`, for its translations.
pub(crate) const MANIFEST: &str = include_str!("../plugin.json");

/// SENAPRED's layers on ArcGIS Online (the Visor Chile Preparado's).
const SENAPRED: &str = "https://services5.arcgis.com/i7S5PSnIJAUcWvSE/ArcGIS/rest/services";
const SENAPRED_CREDIT: &str = "SENAPRED, Visor Chile Preparado (www.visorchilepreparado.cl)";
const USGS_CREDIT: &str = "USGS Earthquake Hazards Program (earthquake.usgs.gov)";
const FIRMS_CREDIT: &str = "NASA FIRMS, VIIRS 375 m (firms.modaps.eosdis.nasa.gov), via Esri Living Atlas";

/// Esri's Living Atlas layer of VIIRS hotspots: the last seven days, worldwide.
const HOTSPOTS: &str =
    "https://services9.arcgis.com/RHVPKKiFTONKtxq3/arcgis/rest/services/Satellite_VIIRS_Thermal_Hotspots_and_Fire_Activity/FeatureServer/0";

/// Chile and its sea, for the earthquakes: `[west, south, east, north]`. It takes in the trench
/// and some of Argentina and Bolivia, whose deep earthquakes are felt in Chile.
const QUAKE_BOX: [f64; 4] = [-80.0, -57.0, -65.0, -17.0];

/// A layer read whole, not in the map view (see [`Source::Arcgis`]).
const WHOLE: f64 = 0.0;

/// A layer in the catalog.
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    /// The command id.
    pub id: &'static str,
    /// The layer's name on the map.
    pub name: &'static str,
    pub credit: &'static str,
    pub source: Source,
    pub style: Style,
}

/// Where a layer's features come from.
#[derive(Clone, Copy, Debug)]
pub enum Source {
    /// An ArcGIS feature layer: read whole ([`WHOLE`]), or in the map view from the map scale
    /// `scale` (its denominator) in, where the most its server gives to one request is `max`.
    Arcgis { layer: &'static str, scale: f64, max: u32 },
    /// The USGS's earthquakes of the last `days`, of magnitude `min_mag` or more.
    Quakes { days: u32, min_mag: f64 },
    /// The VIIRS hotspots of the last `hours` inside Chile.
    Hotspots { hours: u32 },
}

/// How a layer is drawn once added.
#[derive(Clone, Copy, Debug)]
pub enum Style {
    /// RustGIS's own symbol.
    Plain,
    /// One colour.
    Color(&'static str),
    /// Unique values of a field, each with its colour.
    Classes(&'static str, &'static [(&'static str, &'static str)]),
    /// Earthquakes: a marker that grows with the magnitude.
    Quakes,
}

/// Low to high, green to dark red.
const RED: &str = "#b2182b";
const ORANGE: &str = "#ef8a62";
const YELLOW: &str = "#fddb6d";

pub const CATALOG: &[Entry] = &[
    // Earthquakes
    quakes("quakes-day", "Sismos M2,5+ · últimas 24 horas (USGS)", 1, 2.5),
    quakes("quakes-week", "Sismos M2,5+ · últimos 7 días (USGS)", 7, 2.5),
    quakes("quakes-month", "Sismos M2,5+ · últimos 30 días (USGS)", 30, 2.5),
    quakes("quakes-year", "Sismos M4,5+ · últimos 12 meses (USGS)", 365, 4.5),
    // Tsunami
    senapred(
        "tsunami-areas",
        "Tsunami: área a evacuar (SENAPRED)",
        "Amenaza_por_Tsunami_2024/FeatureServer/3",
        WHOLE,
        Style::Color("#4393c3a0"),
    ),
    senapred(
        "tsunami-routes",
        "Tsunami: vías de evacuación (SENAPRED)",
        "Amenaza_por_Tsunami_2024/FeatureServer/1",
        2_000_000.0,
        Style::Color("#1a9850"),
    ),
    senapred(
        "tsunami-meeting-points",
        "Tsunami: puntos de encuentro (SENAPRED)",
        "Amenaza_por_Tsunami_2024/FeatureServer/0",
        WHOLE,
        Style::Color("#1a9850"),
    ),
    senapred(
        "tsunami-safe-line",
        "Tsunami: línea segura (SENAPRED)",
        "Amenaza_por_Tsunami_2024/FeatureServer/2",
        WHOLE,
        Style::Color("#2166ac"),
    ),
    senapred("tsunami-cota30", "Cota 30 m.s.n.m. 2026 (SENAPRED)", "Cota30_2026/FeatureServer/0", WHOLE, Style::Color("#92c5de80")),
    // Volcanoes
    senapred(
        "volcanoes",
        "Volcanes activos: peligrosidad (SENAPRED, SERNAGEOMIN)",
        "AMENAZA_VOLC%C3%81NICA_2024/FeatureServer/3",
        WHOLE,
        Style::Classes(
            "categoria",
            &[
                ("Muy Alta", "#67001f"),
                ("Alta", RED),
                ("Moderada", ORANGE),
                ("Baja", YELLOW),
                ("Muy Baja", "#fff7bc"),
                ("Sin ranking", "#bdbdbd"),
            ],
        ),
    ),
    senapred(
        "volcanic-hazard",
        "Áreas de peligro volcánico (SENAPRED)",
        "AMENAZA_VOLC%C3%81NICA_2024/FeatureServer/2",
        2_000_000.0,
        Style::Classes("peligro", &[("Alto", "#b2182bb0"), ("Medio", "#ef8a62b0"), ("Bajo", "#fddb6db0")]),
    ),
    senapred(
        "volcanic-evacuation",
        "Volcanes: áreas de evacuación (SENAPRED)",
        "%C3%81rea_de_Evacuaci%C3%B3n_Volcanes/FeatureServer/0",
        WHOLE,
        Style::Color("#f4a58280"),
    ),
    senapred(
        "volcanic-routes",
        "Volcanes: vías de evacuación (SENAPRED)",
        "AMENAZA_VOLC%C3%81NICA_2024/FeatureServer/1",
        WHOLE,
        Style::Color("#1a9850"),
    ),
    senapred(
        "volcanic-meeting-points",
        "Volcanes: puntos de encuentro (SENAPRED)",
        "AMENAZA_VOLC%C3%81NICA_2024/FeatureServer/0",
        WHOLE,
        Style::Color("#1a9850"),
    ),
    // Wildfires
    hotspots("hotspots-day", "Focos de calor · últimas 24 horas (VIIRS)", 24),
    hotspots("hotspots-2days", "Focos de calor · últimas 48 horas (VIIRS)", 48),
    hotspots("hotspots-week", "Focos de calor · últimos 7 días (VIIRS)", 168),
    senapred(
        "fire-recurrence",
        "Recurrencia de incendios forestales 2020–2024 (SENAPRED)",
        "Amenaza_por_Incendio_Forestal_2024/FeatureServer/0",
        500_000.0,
        Style::Classes(
            "recurrencia",
            &[("Muy alta", "#99000dc0"), ("Alta", "#e34a33c0"), ("Media", "#fc8d59c0"), ("Baja", "#fdcc8ac0"), ("Muy baja", "#fef0d9c0")],
        ),
    ),
];

const fn quakes(id: &'static str, name: &'static str, days: u32, min_mag: f64) -> Entry {
    Entry { id, name, credit: USGS_CREDIT, source: Source::Quakes { days, min_mag }, style: Style::Quakes }
}

const fn hotspots(id: &'static str, name: &'static str, hours: u32) -> Entry {
    Entry { id, name, credit: FIRMS_CREDIT, source: Source::Hotspots { hours }, style: Style::Color("#e31a1c") }
}

/// A SENAPRED layer. Every one gives 2,000 features to a request, or more.
const fn senapred(id: &'static str, name: &'static str, layer: &'static str, scale: f64, style: Style) -> Entry {
    Entry { id, name, credit: SENAPRED_CREDIT, source: Source::Arcgis { layer, scale, max: 2000 }, style }
}

/// Runs a command.
pub(crate) fn handle(req: Request) -> Result<Value, String> {
    if let Some(e) = CATALOG.iter().find(|e| e.id == req.id) {
        return add(&req, e);
    }
    match req.id.as_str() {
        "about" => about(&req),
        other => Err(format!("Riesgos Chile has no command '{other}'")),
    }
}

#[cfg(target_arch = "wasm32")]
rustgis_plugin::export!(handle);

/// The map view: its Web Mercator box `[west, south, east, north]` and metres per pixel, from
/// `host.view`'s centre and scale: the box a screen of about 1600 × 1000 pixels at 96 dpi shows
/// at that scale, which covers the view with a margin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub bbox: [f64; 4],
    pub m_per_px: f64,
}

pub fn view_of(view: &Value) -> Option<View> {
    let c = view.get("center")?.as_array()?;
    let (lon, lat) = (c.first()?.as_f64()?, c.get(1)?.as_f64()?);
    let scale = view.get("scale")?.as_f64()?;
    if !(lon.is_finite() && lat.is_finite() && scale.is_finite()) || scale <= 0.0 {
        return None;
    }
    let (x, y) = mercator(lon, lat.clamp(-85.05, 85.05));
    let m_per_px = scale * 0.0254 / 96.0;
    let (half_w, half_h) = (800.0 * m_per_px, 500.0 * m_per_px);
    Some(View { bbox: [x - half_w, y - half_h, x + half_w, y + half_h], m_per_px })
}

/// Longitude/latitude in degrees to Web Mercator metres.
fn mercator(lon: f64, lat: f64) -> (f64, f64) {
    const R: f64 = 6_378_137.0;
    let x = lon.to_radians() * R;
    let y = ((std::f64::consts::FRAC_PI_4 + lat.to_radians() / 2.0).tan()).ln() * R;
    (x, y)
}

/// A query value percent-encoded: letters, digits and `-_.~,:` stay, everything else is `%XX`.
/// (Commas and colons are left so Chile's outline fits in the 2,048 characters ArcGIS Online
/// takes in an address.)
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-_.~,:".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

/// The address of an entry's features as GeoJSON. An ArcGIS layer read in the map view needs the
/// view: the features in it, generalised to about a pixel.
pub fn url(e: &Entry, view: Option<View>) -> String {
    match e.source {
        Source::Arcgis { layer, .. } => {
            let mut u =
                format!("{SENAPRED}/{layer}/query?where=1%3D1&outFields=*&returnGeometry=true&outSR=4326&geometryPrecision=5&f=geojson");
            match view {
                Some(v) => {
                    let [w, s, e, n] = v.bbox;
                    // The offset is in the answer's degrees: a pixel's worth at the equator's scale.
                    let offset = v.m_per_px / 111_320.0;
                    u.push_str(&format!(
                        "&geometry={w:.0},{s:.0},{e:.0},{n:.0}&geometryType=esriGeometryEnvelope&inSR=3857&spatialRel=esriSpatialRelIntersects&maxAllowableOffset={offset:.7}"
                    ));
                }
                // About 10 m: a whole layer is a few megabytes at most.
                None => u.push_str("&maxAllowableOffset=0.0001"),
            }
            u
        }
        Source::Quakes { days, min_mag } => {
            let [w, s, e, n] = QUAKE_BOX;
            format!(
                "https://earthquake.usgs.gov/fdsnws/event/1/query?format=geojson&starttime=NOW-{days}days&minmagnitude={min_mag}&minlatitude={s}&maxlatitude={n}&minlongitude={w}&maxlongitude={e}&orderby=time"
            )
        }
        Source::Hotspots { hours } => format!(
            "{HOTSPOTS}/query?where={}&geometry={}&geometryType=esriGeometryPolygon&inSR=4326&spatialRel=esriSpatialRelIntersects&outFields={}&returnGeometry=true&outSR=4326&f=geojson",
            encode(&format!("hours_old<={hours}")),
            encode(outline::CHILE),
            encode("acq_date,acq_time,satellite,confidence,frp,daynight,hours_old"),
        ),
    }
}

/// The map scale a layer is read in the map view from, if it is.
fn in_view(e: &Entry) -> Option<f64> {
    match e.source {
        Source::Arcgis { scale, .. } if scale > WHOLE => Some(scale),
        _ => None,
    }
}

/// A map scale's denominator as people write it: `2,000,000` in English, `2.000.000` in Spanish.
fn scale_text(scale: f64, lang: &str) -> String {
    let digits = format!("{:.0}", scale);
    let sep = if lang.starts_with("es") { '.' } else { ',' };
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(sep);
        }
        out.push(c);
    }
    out
}

fn add(req: &Request, e: &Entry) -> Result<Value, String> {
    let name = &[("name", e.name)];
    let view = if let Some(max_scale) = in_view(e) {
        let view = host::view()?;
        // A 1% margin: a view set to the scale itself reports it with some rounding.
        if view.get("scale").and_then(|s| s.as_f64()).is_none_or(|s| s > max_scale * 1.01) {
            return Err(req.tr(
                MANIFEST,
                "{name} is read in the map view: zoom in to 1:{scale} or closer first.",
                &[("name", e.name), ("scale", &scale_text(max_scale, &req.lang))],
            ));
        }
        Some(view_of(&view).ok_or("the map has no view to read the features in")?)
    } else {
        None
    };
    let address = url(e, view);
    let read = match fetch::fetch_and_add(&address, e.name)? {
        Some(Fetched::Pending) => {
            host::notify(&req.tr(MANIFEST, "Downloading {name}; it is added when it lands.", name));
            return Ok(json!({"pending": true}));
        }
        // `host.addLayer` doesn't count the features: the layer list does.
        Some(Fetched::Added(out)) => Ok(match (out.get("layer").cloned(), host::layers()) {
            (Some(layer), Ok(layers)) => {
                let n = layers.as_array().and_then(|ls| ls.iter().find(|l| l["id"] == layer)).and_then(|l| l["features"].as_u64());
                json!({"layer": layer, "features": n})
            }
            _ => out,
        }),
        None => host::run("layer.addUrl", json!({"url": address, "name": e.name, "zoom": view.is_none()})),
    };
    let out = match read {
        Ok(out) => out,
        // The web app downloads only from its own Add Data dialog.
        Err(err) if err.contains("web app") => {
            return Err(req.tr(
                MANIFEST,
                "In the web app, add {name} with Home › Add Data › From URL and this address: {url}",
                &[("name", e.name), ("url", &address)],
            ));
        }
        Err(err) => return Err(err),
    };
    // `layer.addUrl` in the desktop app: the layer lands after the command, in RustGIS's style.
    if out.get("pending").and_then(Value::as_bool) == Some(true) {
        host::notify(&req.tr(MANIFEST, "Downloading {name}; it is added when it lands.", name));
        return Ok(out);
    }
    // No features: nothing happened in the period. `layer.addUrl` adds them as a table,
    // `host.addLayer` as an empty layer.
    let empty = out.get("table").or_else(|| out.get("layer").filter(|_| out.get("features").and_then(Value::as_u64) == Some(0)));
    if let Some(added) = empty.cloned() {
        let _ = host::run("layer.remove", json!({"layer": added}));
        host::notify(&req.tr(MANIFEST, "{name}: nothing to show right now.", name));
        return Ok(json!({"features": 0}));
    }
    if let Some(layer) = out.get("layer").cloned() {
        // A style that doesn't take (an older RustGIS) leaves RustGIS's own.
        let _ = style(&layer, e.style);
        // `host.addLayer` doesn't zoom to what it adds.
        if view.is_none() {
            let _ = host::run("layer.zoomTo", json!({"layer": layer}));
        }
    }
    let n = out.get("features").and_then(Value::as_u64).unwrap_or(0);
    let limit = match e.source {
        Source::Arcgis { max, .. } => u64::from(max),
        _ => u64::MAX,
    };
    if view.is_some() && n >= limit {
        host::notify(&req.tr(
            MANIFEST,
            "{name}: the map view holds more than the {n} features the service gives at once, so the layer is incomplete. Zoom in and choose it again.",
            &[("name", e.name), ("n", &limit.to_string())],
        ));
    } else {
        let n = n.to_string();
        host::notify(&req.tr(
            MANIFEST,
            if view.is_some() { "Added {name} in the map view: {n} features." } else { "Added {name}: {n} features." },
            &[("name", e.name), ("n", &n)],
        ));
    }
    Ok(out)
}

/// The commands that draw a layer in its style.
pub fn style_commands(layer: &Value, style: Style) -> Vec<(&'static str, Value)> {
    match style {
        Style::Plain => Vec::new(),
        Style::Color(c) => vec![("layer.color", json!({"layer": layer, "color": c}))],
        Style::Classes(field, classes) => std::iter::once(("layer.symbology", json!({"layer": layer, "type": "unique", "field": field})))
            .chain(classes.iter().map(|(v, c)| ("layer.classStyle", json!({"layer": layer, "value": v, "color": c}))))
            .collect(),
        Style::Quakes => vec![
            ("layer.color", json!({"layer": layer, "color": "#d7301fc0"})),
            // About 4 points at magnitude 2.5, 16 at 7.
            ("layer.symbolBy", json!({"layer": layer, "size": "\"mag\" * \"mag\" / 3"})),
        ],
    }
}

fn style(layer: &Value, style: Style) -> Result<(), String> {
    for (command, params) in style_commands(layer, style) {
        host::run(command, params)?;
    }
    Ok(())
}

fn about(req: &Request) -> Result<Value, String> {
    let n = CATALOG.len().to_string();
    host::notify(&req.tr(
        MANIFEST,
        "Riesgos Chile: {n} layers of earthquakes (USGS), wildfire hotspots (NASA FIRMS) and tsunami, volcanic and wildfire hazard (SENAPRED). Each layer's credit names its source. They are for information: in an emergency, follow SENAPRED's instructions.",
        &[("n", &n)],
    ));
    let layers: Vec<Value> = CATALOG.iter().map(|e| json!({"id": e.id, "name": e.name, "credit": e.credit})).collect();
    Ok(json!({"layers": layers}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustgis_plugin::serde_json;

    fn entry(id: &str) -> &'static Entry {
        CATALOG.iter().find(|e| e.id == id).unwrap()
    }

    #[test]
    fn catalog_and_manifest_agree() {
        let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        let commands: Vec<&str> = manifest["commands"].as_array().unwrap().iter().filter_map(|c| c["id"].as_str()).collect();
        for e in CATALOG {
            assert!(commands.contains(&e.id), "{} has no command", e.id);
            assert!(!e.name.is_empty() && !e.credit.is_empty(), "{}", e.id);
            assert_eq!(CATALOG.iter().filter(|o| o.id == e.id).count(), 1, "{}", e.id);
        }
        for c in &commands {
            assert!(CATALOG.iter().any(|e| e.id == *c) || *c == "about", "{c} isn't handled");
        }
        assert_eq!(manifest["permissions"], json!(["edit", "network"]));
        // Every item of the tab is a command.
        for g in manifest["tab"]["groups"].as_array().unwrap() {
            for it in g["items"].as_array().unwrap() {
                match it {
                    Value::String(id) => assert!(commands.contains(&id.as_str()), "{id}"),
                    menu => {
                        for id in menu["items"].as_array().unwrap() {
                            assert!(commands.contains(&id.as_str().unwrap()), "{id}");
                        }
                    }
                }
            }
        }
        // And every command is in the tab.
        let tab = manifest["tab"].to_string();
        for c in commands {
            assert!(tab.contains(&format!("\"{c}\"")), "{c} isn't in the tab");
        }
    }

    #[test]
    fn spanish_covers_the_interface() {
        let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        let es = manifest["i18n"]["es"].as_object().unwrap();
        let mut texts: Vec<&str> = Vec::new();
        for c in manifest["commands"].as_array().unwrap() {
            texts.extend(["label", "help"].iter().map(|k| c[*k].as_str().unwrap()));
        }
        for g in manifest["tab"]["groups"].as_array().unwrap() {
            texts.push(g["label"].as_str().unwrap());
            for it in g["items"].as_array().unwrap() {
                if it.is_object() {
                    texts.extend(["menu", "help"].iter().map(|k| it[*k].as_str().unwrap()));
                }
            }
        }
        texts.push(manifest["tab"]["label"].as_str().unwrap());
        for s in texts {
            assert!(es.contains_key(s), "no Spanish for {s:?}");
        }
        // The messages the commands show.
        // (Up to the tests, which name the call themselves.)
        let src = include_str!("lib.rs").split("#[cfg(test)]").next().unwrap();
        for s in src.split("req.tr(").skip(1) {
            let msg = s.split('"').nth(1).unwrap();
            if !msg.is_empty() {
                assert!(es.contains_key(msg), "no Spanish for {msg:?}");
            }
        }
    }

    #[test]
    fn earthquakes_of_a_period() {
        let u = url(entry("quakes-week"), None);
        assert_eq!(
            u,
            "https://earthquake.usgs.gov/fdsnws/event/1/query?format=geojson&starttime=NOW-7days&minmagnitude=2.5&minlatitude=-57&maxlatitude=-17&minlongitude=-80&maxlongitude=-65&orderby=time"
        );
        assert!(url(entry("quakes-year"), None).contains("starttime=NOW-365days&minmagnitude=4.5&"));
    }

    #[test]
    fn hotspots_inside_chile_fit_an_address() {
        let u = url(entry("hotspots-2days"), None);
        assert!(u.starts_with(HOTSPOTS) && u.contains("/query?where=hours_old%3C%3D48&geometry=%7B%22rings%22:%5B%5B%5B-"), "{u}");
        assert!(u.contains("&geometryType=esriGeometryPolygon&inSR=4326&") && u.ends_with("&outSR=4326&f=geojson"), "{u}");
        // ArcGIS Online answers a longer address with a 404.
        assert!(u.len() < 2048, "{} characters", u.len());
        let rings: Value = serde_json::from_str(outline::CHILE).unwrap();
        for ring in rings["rings"].as_array().unwrap() {
            let ring = ring.as_array().unwrap();
            assert!(ring.len() >= 4 && ring.first() == ring.last());
        }
    }

    #[test]
    fn senapred_layers_whole_or_in_view() {
        let whole = url(entry("tsunami-areas"), None);
        assert_eq!(
            whole,
            format!(
                "{SENAPRED}/Amenaza_por_Tsunami_2024/FeatureServer/3/query?where=1%3D1&outFields=*&returnGeometry=true&outSR=4326&geometryPrecision=5&f=geojson&maxAllowableOffset=0.0001"
            )
        );
        assert_eq!((in_view(entry("tsunami-areas")), in_view(entry("tsunami-routes"))), (None, Some(2_000_000.0)));
        // Its grid has some 1,300 cells in a view at 1:500,000 in the south-centre, 3,000 at 1:1,000,000.
        assert_eq!(in_view(entry("fire-recurrence")), Some(500_000.0));
        // Concepción at 1:500,000.
        let v = view_of(&json!({"center": [-73.05, -36.82], "scale": 500000.0})).unwrap();
        let u = url(entry("volcanic-hazard"), Some(v));
        assert!(
            u.contains("/AMENAZA_VOLC%C3%81NICA_2024/FeatureServer/2/query?") && u.contains("&geometry=-8") && u.contains("&inSR=3857&"),
            "{u}"
        );
        assert!(u.ends_with("&maxAllowableOffset=0.0011884"), "{u}");
        assert_eq!(scale_text(2_000_000.0, "es"), "2.000.000");
        assert_eq!(scale_text(500_000.0, "en"), "500,000");
        assert!(view_of(&json!({"center": [0.0, 0.0]})).is_none());
    }

    #[test]
    fn styles() {
        let layer = json!(3);
        let c = style_commands(&layer, entry("volcanic-hazard").style);
        assert_eq!(c[0], ("layer.symbology", json!({"layer": 3, "type": "unique", "field": "peligro"})));
        assert_eq!(c.len(), 4);
        assert_eq!(c[1], ("layer.classStyle", json!({"layer": 3, "value": "Alto", "color": "#b2182bb0"})));
        let q = style_commands(&layer, Style::Quakes);
        assert_eq!(q[1].0, "layer.symbolBy");
        assert!(style_commands(&layer, Style::Plain).is_empty());
    }
}
