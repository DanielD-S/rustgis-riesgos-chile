"""Checks that every layer's service still answers what the plugin asks it.

    python scripts/check_services.py                    # the probes from `cargo run --example probes`
    python scripts/check_services.py --report out.md    # and a Markdown report of what failed

Each probe (see examples/probes.rs) is one of the plugin's own requests, cut to one feature: it
must come back as GeoJSON, with a feature that has its geometry (the earthquakes and hotspots of a
period may be none). A request that fails is tried again twice, a while later, before it counts:
the services are slow at times.

Exits 1 when a probe fails. The weekly Services workflow turns the report into an issue, and
closes it when every service answers again.
"""

import argparse
import json
import os
import pathlib
import subprocess
import sys
import time
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
AGENT = "RustGIS Riesgos Chile plugin (services check; github.com/DanielD-S/rustgis-riesgos-chile)"
TIMEOUT = 90
WAITS = (10, 30)  # before the second and third tries


def probes():
    env = dict(os.environ)
    if os.name == "nt":
        # The library names RustGIS's host imports, which a native Windows link can't resolve
        # (Linux leaves them out).
        env["RUSTFLAGS"] = (env.get("RUSTFLAGS", "") + " -C link-arg=/FORCE:UNRESOLVED").strip()
    out = subprocess.run(
        ["cargo", "run", "--quiet", "--locked", "--example", "probes"],
        cwd=ROOT, env=env, check=True, capture_output=True, text=True, encoding="utf-8",
    ).stdout
    return [json.loads(line) for line in out.splitlines() if line.strip()]


def problem(probe):
    """What is wrong with a probe's answer, or None."""
    req = urllib.request.Request(probe["url"], headers={"User-Agent": AGENT})
    try:
        with urllib.request.urlopen(req, timeout=TIMEOUT) as r:
            body = r.read()
    except Exception as e:  # noqa: BLE001 - any failure is the service's
        return f"no answer: {e}"
    try:
        doc = json.loads(body)
    except ValueError:
        return f"not JSON: {body[:80]!r}"
    if "error" in doc:
        return f"error: {json.dumps(doc['error'])[:120]}"
    features = doc.get("features")
    if doc.get("type") != "FeatureCollection" or not isinstance(features, list):
        return "not GeoJSON"
    if not features:
        return None if probe["mayBeEmpty"] else "no features"
    if not features[0].get("geometry"):
        return "features come without geometry"
    return None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--report", help="write a Markdown report of what failed here")
    args = ap.parse_args()

    failed = []
    all_probes = probes()
    for p in all_probes:
        why = problem(p)
        for wait in WAITS:
            if why is None:
                break
            time.sleep(wait)
            why = problem(p)
        print(f"{'ok  ' if why is None else 'FAIL'} {p['entry']}: {why or p['name']}")
        if why:
            failed.append((p, why))

    if args.report:
        stamp = time.strftime("%Y-%m-%d %H:%M UTC", time.gmtime())
        lines = [
            f"**{len(all_probes) - len(failed)} of {len(all_probes)} layers answered.**",
            "",
            f"Checked {stamp} by `scripts/check_services.py`.",
            "",
        ]
        if failed:
            lines += ["| Command | Layer | Request | Problem |", "|---|---|---|---|"]
            lines += [f"| `{p['entry']}` | {p['name']} | [request]({p['url']}) | {why} |" for p, why in failed]
            lines += ["", "A layer whose service moved needs its new address in `src/lib.rs`; one whose service is gone leaves the catalog until there is another."]
        pathlib.Path(args.report).write_text("\n".join(lines) + "\n", encoding="utf-8")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
