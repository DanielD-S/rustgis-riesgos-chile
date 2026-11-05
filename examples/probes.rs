//! The requests `scripts/check_services.py` sends to see that every layer's service still
//! answers, one JSON object a line, built with the plugin's own addresses (`url`) so the check
//! asks what the plugin asks:
//!
//!     cargo run --quiet --example probes
//!
//! Each is cut to one feature (`resultRecordCount=1`, the USGS's `limit=1`), which must come as
//! GeoJSON with its geometry. The earthquakes and hotspots of a period may be none: their answer
//! must be GeoJSON, empty or not.

use riesgos_chile::{CATALOG, Source, url};
use rustgis_plugin::json;

fn main() {
    for e in CATALOG {
        let (u, may_be_empty) = match e.source {
            Source::Quakes { .. } => (format!("{}&limit=1", url(e, None)), true),
            Source::Hotspots { .. } => (format!("{}&resultRecordCount=1", url(e, None)), true),
            Source::Arcgis { .. } => (format!("{}&resultRecordCount=1", url(e, None)), false),
        };
        println!("{}", json!({"entry": e.id, "name": e.name, "url": u, "mayBeEmpty": may_be_empty}));
    }
}
