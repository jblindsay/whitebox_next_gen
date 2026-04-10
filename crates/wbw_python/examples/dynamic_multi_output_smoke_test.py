import json
import tempfile
from pathlib import Path

import whitebox_workflows as wb


def write_square_geojson(path: Path) -> None:
    path.write_text(
        json.dumps(
            {
                "type": "FeatureCollection",
                "features": [
                    {
                        "type": "Feature",
                        "properties": {"id": 1},
                        "geometry": {
                            "type": "Polygon",
                            "coordinates": [[
                                [0.0, 0.0],
                                [4.0, 0.0],
                                [4.0, 4.0],
                                [0.0, 4.0],
                                [0.0, 0.0],
                            ]],
                        },
                    }
                ],
            }
        ),
        encoding="utf-8",
    )


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="wbw_multi_output_") as td:
        root = Path(td)
        polygon = root / "base.geojson"
        write_square_geojson(polygon)

        wbe = wb.WbEnvironment()
        wbe.working_directory = str(root)
        base_vector = wbe.read_vector(str(polygon))

        dem = wbe.new_raster_from_base_vector(
            base=base_vector,
            cell_size=1.0,
            out_val=5.0,
            data_type="float",
            output_path=str(root / "dem.tif"),
        )

        intensity_path = root / "intensity.tif"
        hue_path = root / "hue.tif"
        saturation_path = root / "saturation.tif"

        outputs = wbe.remote_sensing.rgb_to_ihs(
            red=dem,
            green=dem,
            blue=dem,
            intensity_output=str(intensity_path),
            hue_output=str(hue_path),
            saturation_output=str(saturation_path),
        )

        assert isinstance(outputs, (list, tuple)), type(outputs)
        assert len(outputs) == 3, outputs
        returned_paths = set()
        for raster in outputs:
            assert hasattr(raster, "file_path"), type(raster)
            returned_paths.add(Path(raster.file_path))

        assert returned_paths == {intensity_path, hue_path, saturation_path}

    print("dynamic multi-output smoke test passed")


if __name__ == "__main__":
    main()