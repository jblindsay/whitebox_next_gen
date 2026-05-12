"""Simple TopoJSON roundtrip example for WbW-Py.

This script reads a vector layer, writes it as TopoJSON, and reads it back.
"""

from pathlib import Path
import whitebox_workflows as wb


def main() -> None:
    wbe = wb.WbEnvironment()

    src = Path("input_roads.gpkg")
    topo = Path("output_roads.topojson")
    back = Path("output_roads_back.gpkg")

    roads = wbe.read_vector(str(src))
    wbe.write_vector(roads, str(topo))

    roads_topo = wbe.read_vector(str(topo))
    wbe.write_vector(roads_topo, str(back))

    print(f"wrote TopoJSON: {topo}")
    print(f"wrote roundtrip GeoPackage: {back}")


if __name__ == "__main__":
    main()
