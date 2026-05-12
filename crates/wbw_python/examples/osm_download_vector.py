"""Download OSM vector features with wbw_python.

Example: fetch trail centerlines using a projected Web Mercator bbox,
transforming query extents to EPSG:4326 internally.
"""

from wbw import WbEnvironment


def main() -> None:
    wbe = WbEnvironment()

    result = wbe.run_tool(
        "download_osm_vector",
        {
            "west": -8983000.0,
            "south": 5382000.0,
            "east": -8965000.0,
            "north": 5393000.0,
            "input_extent_epsg": 3857,
            "filter_preset": "trails",
            "overpass_profile": "kumi",
            "include_points": False,
            "include_lines": True,
            "include_polygons": False,
            "timeout_seconds": 30,
            "max_elements": 50000,
            "output": "kitchener_trails.geojson",
        },
    )

    print(f"wrote output: {result.outputs.get('path')}")


if __name__ == "__main__":
    main()
