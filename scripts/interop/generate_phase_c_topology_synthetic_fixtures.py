#!/usr/bin/env python3
"""Generate minimal Phase C topology stress corpus fixtures.

This script creates one synthetic GeoJSON fixture per topology pathology class
listed in the interop plan (TC01-TC07).
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List

ROOT = Path(__file__).resolve().parents[2]
SYNTHETIC_OUT_DIR = ROOT / "artifacts" / "interop" / "topology" / "corpus" / "synthetic"
COMPLEX_OUT_DIR = ROOT / "artifacts" / "interop" / "topology" / "corpus" / "complex"


def feature(geometry: Dict[str, Any], props: Dict[str, Any]) -> Dict[str, Any]:
    return {
        "type": "Feature",
        "properties": props,
        "geometry": geometry,
    }


def write_fc(path: Path, features: List[Dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fc = {
        "type": "FeatureCollection",
        "name": path.stem,
        "crs": {
            "type": "name",
            "properties": {"name": "EPSG:4326"},
        },
        "features": features,
    }
    path.write_text(json.dumps(fc, indent=2) + "\n", encoding="utf-8")


def tc01_bow_tie() -> List[Dict[str, Any]]:
    # Self-intersecting polygon (bow-tie).
    return [
        feature(
            {
                "type": "Polygon",
                "coordinates": [
                    [
                        [0.0, 0.0],
                        [2.0, 2.0],
                        [0.0, 2.0],
                        [2.0, 0.0],
                        [0.0, 0.0],
                    ]
                ],
            },
            {"id": 1, "class": "self_intersection"},
        )
    ]


def tc02_sliver() -> List[Dict[str, Any]]:
    # Nearly-coincident edges creating a very thin sliver.
    return [
        feature(
            {
                "type": "Polygon",
                "coordinates": [
                    [
                        [0.0, 0.0],
                        [2.0, 0.0],
                        [2.0, 0.000001],
                        [1.0, 0.0000015],
                        [0.0, 0.000001],
                        [0.0, 0.0],
                    ]
                ],
            },
            {"id": 1, "class": "sliver"},
        )
    ]


def tc03_ring_orientation() -> List[Dict[str, Any]]:
    # Outer ring clockwise and hole counter-clockwise (anomaly pattern).
    return [
        feature(
            {
                "type": "Polygon",
                "coordinates": [
                    [
                        [0.0, 0.0],
                        [0.0, 4.0],
                        [4.0, 4.0],
                        [4.0, 0.0],
                        [0.0, 0.0],
                    ],
                    [
                        [1.0, 1.0],
                        [3.0, 1.0],
                        [3.0, 3.0],
                        [1.0, 3.0],
                        [1.0, 1.0],
                    ],
                ],
            },
            {"id": 1, "class": "ring_orientation"},
        )
    ]


def tc04_duplicate_vertices() -> List[Dict[str, Any]]:
    # Line with duplicate and near-duplicate vertices.
    return [
        feature(
            {
                "type": "LineString",
                "coordinates": [
                    [0.0, 0.0],
                    [1.0, 1.0],
                    [1.0, 1.0],
                    [1.0000000005, 1.0000000005],
                    [2.0, 2.0],
                    [3.0, 3.0],
                ],
            },
            {"id": 1, "class": "duplicate_vertices"},
        )
    ]


def tc05_tiny_gap_overlap() -> List[Dict[str, Any]]:
    # Two adjacent polygons with tiny gap and tiny overlap in one fixture.
    return [
        feature(
            {
                "type": "Polygon",
                "coordinates": [
                    [
                        [0.0, 0.0],
                        [2.0, 0.0],
                        [2.0, 2.0],
                        [0.0, 2.0],
                        [0.0, 0.0],
                    ]
                ],
            },
            {"id": 1, "class": "fabric_a"},
        ),
        feature(
            {
                "type": "Polygon",
                "coordinates": [
                    [
                        [2.0000002, 0.0],
                        [4.0, 0.0],
                        [4.0, 2.0],
                        [2.0000002, 2.0],
                        [2.0000002, 0.0],
                    ]
                ],
            },
            {"id": 2, "class": "tiny_gap"},
        ),
        feature(
            {
                "type": "Polygon",
                "coordinates": [
                    [
                        [1.9999998, 2.2],
                        [4.0, 2.2],
                        [4.0, 4.0],
                        [1.9999998, 4.0],
                        [1.9999998, 2.2],
                    ]
                ],
            },
            {"id": 3, "class": "tiny_overlap"},
        ),
    ]


def tc06_point_touch() -> List[Dict[str, Any]]:
    # Polygons touching at a single corner point.
    return [
        feature(
            {
                "type": "Polygon",
                "coordinates": [
                    [
                        [0.0, 0.0],
                        [1.0, 0.0],
                        [1.0, 1.0],
                        [0.0, 1.0],
                        [0.0, 0.0],
                    ]
                ],
            },
            {"id": 1, "class": "point_touch_a"},
        ),
        feature(
            {
                "type": "Polygon",
                "coordinates": [
                    [
                        [1.0, 1.0],
                        [2.0, 1.0],
                        [2.0, 2.0],
                        [1.0, 2.0],
                        [1.0, 1.0],
                    ]
                ],
            },
            {"id": 2, "class": "point_touch_b"},
        ),
    ]


def tc07_multipart_edge_cases() -> List[Dict[str, Any]]:
    # MultiPolygon with one normal part and one very small part.
    return [
        feature(
            {
                "type": "MultiPolygon",
                "coordinates": [
                    [
                        [
                            [0.0, 0.0],
                            [2.0, 0.0],
                            [2.0, 2.0],
                            [0.0, 2.0],
                            [0.0, 0.0],
                        ]
                    ],
                    [
                        [
                            [3.0, 3.0],
                            [3.000001, 3.0],
                            [3.000001, 3.000001],
                            [3.0, 3.000001],
                            [3.0, 3.0],
                        ]
                    ],
                ],
            },
            {"id": 1, "class": "multipart_edge"},
        )
    ]


def build_complex_from(base: List[Dict[str, Any]], copies: int, x_step: float, y_step: float) -> List[Dict[str, Any]]:
    out: List[Dict[str, Any]] = []

    def shift_coords(obj: Any, dx: float, dy: float) -> Any:
        if isinstance(obj, list):
            if obj and isinstance(obj[0], (int, float)) and len(obj) >= 2:
                return [obj[0] + dx, obj[1] + dy]
            return [shift_coords(v, dx, dy) for v in obj]
        return obj

    for i in range(copies):
        dx = i * x_step
        dy = i * y_step
        for feat in base:
            geom = feat["geometry"]
            shifted_geom = {
                "type": geom["type"],
                "coordinates": shift_coords(geom["coordinates"], dx, dy),
            }
            props = dict(feat.get("properties", {}))
            props["copy_id"] = i + 1
            out.append(feature(shifted_geom, props))
    return out


def write_fixture_set(out_dir: Path, fixture_builders: Dict[str, List[Dict[str, Any]]], kind: str) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    for filename, features in fixture_builders.items():
        write_fc(out_dir / filename, features)

    manifest = {
        "phase": "C",
        "kind": kind,
        "crs": "EPSG:4326",
        "fixture_count": len(fixture_builders),
        "fixtures": sorted(fixture_builders.keys()),
    }
    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def main() -> None:
    fixtures = {
        "TC01_self_intersection_bow_tie.geojson": tc01_bow_tie(),
        "TC02_nearly_coincident_edges_sliver.geojson": tc02_sliver(),
        "TC03_ring_orientation_anomalies.geojson": tc03_ring_orientation(),
        "TC04_duplicate_near_duplicate_vertices.geojson": tc04_duplicate_vertices(),
        "TC05_tiny_gaps_and_overlaps.geojson": tc05_tiny_gap_overlap(),
        "TC06_point_touch_boundaries.geojson": tc06_point_touch(),
        "TC07_multipart_edge_cases.geojson": tc07_multipart_edge_cases(),
    }

    complex_fixtures = {
        name: build_complex_from(features, copies=6, x_step=0.02, y_step=0.015)
        for name, features in fixtures.items()
    }

    write_fixture_set(SYNTHETIC_OUT_DIR, fixtures, kind="synthetic")
    write_fixture_set(COMPLEX_OUT_DIR, complex_fixtures, kind="complex")

    print(f"wrote {len(fixtures)} synthetic fixtures to {SYNTHETIC_OUT_DIR}")
    print(f"wrote {len(complex_fixtures)} complex fixtures to {COMPLEX_OUT_DIR}")


if __name__ == "__main__":
    main()
