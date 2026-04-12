from __future__ import annotations

from pathlib import Path
import tempfile

import whitebox_workflows as wb


def _maybe_import_numpy():
    try:
        import numpy as np  # type: ignore

        return np
    except ImportError:
        return None


def main() -> None:
    np = _maybe_import_numpy()
    if np is None:
        print("skipped: numpy not installed")
        return

    with tempfile.TemporaryDirectory(prefix="wbw_py_lidar_numpy_") as td:
        tmp_dir = Path(td)
        csv_path = tmp_dir / "points.csv"
        csv_path.write_text(
            "\n".join(
                [
                    "x,y,z,i,c,rn,nr,sa,time,r,g,b",
                    "0.0,0.0,10.0,100,2,1,1,0,1.25,1000,2000,3000",
                    "1.0,1.0,12.0,150,2,1,2,3,2.50,1200,2200,3200",
                ]
            )
            + "\n",
            encoding="utf-8",
        )

        las_path = tmp_dir / "points.las"

        wbe = wb.WbEnvironment()
        wbe.ascii_to_las(
            input_ascii_files=[str(csv_path)],
            pattern="x,y,z,i,c,rn,nr,sa,time,r,g,b",
            epsg_code=4326,
        )

        if not las_path.exists():
            raise AssertionError(f"expected LAS output missing: {las_path}")

        lidar = wbe.read_lidar(str(las_path))
        if lidar.point_count != 2:
            raise AssertionError(f"expected point_count=2, got {lidar.point_count}")

        cols = ["x", "y", "z", "classification"]
        arr = lidar.to_numpy(cols=cols)
        if tuple(arr.shape) != (2, 4):
            raise AssertionError(f"unexpected array shape: {arr.shape}")

        arr[:, 3] = 6
        edited_path = tmp_dir / "points_reclass.laz"
        edited = wb.Lidar.from_numpy(
            arr,
            base=lidar,
            output_path=str(edited_path),
            cols=cols,
        )

        check = edited.to_numpy(cols=["classification"])
        if not np.all(check[:, 0] == 6):
            raise AssertionError(f"classification roundtrip mismatch: {check[:, 0]}")

        chunks = lidar.to_numpy_chunks(chunk_size=1, cols=cols)
        if len(chunks) != 2:
            raise AssertionError(f"expected 2 chunks, got {len(chunks)}")

        for chunk in chunks:
            chunk[:, 3] = 7

        chunked_edited_path = tmp_dir / "points_reclass_chunked.laz"
        chunked_edited = wb.Lidar.from_numpy_chunks(
            chunks,
            base=lidar,
            output_path=str(chunked_edited_path),
            cols=cols,
        )

        check_chunked = chunked_edited.to_numpy(cols=["classification"])
        if not np.all(check_chunked[:, 0] == 7):
            raise AssertionError(
                f"chunked classification roundtrip mismatch: {check_chunked[:, 0]}"
            )

        print("lidar numpy roundtrip smoke test passed")


if __name__ == "__main__":
    main()
