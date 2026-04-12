from __future__ import annotations

import tempfile
from pathlib import Path

import whitebox_workflows as wb


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="wbw_py_lidar_chunked_") as td:
        tmp_dir = Path(td)

        csv_path = tmp_dir / "points.csv"
        csv_path.write_text(
            "\n".join(
                [
                    "x,y,z,i,c,rn,nr,sa,time,r,g,b",
                    "0.0,0.0,10.0,100,2,1,1,0,1.25,1000,2000,3000",
                    "1.0,1.0,12.0,150,2,1,2,3,2.50,1200,2200,3200",
                    "2.0,1.5,15.0,180,1,1,1,2,3.20,1300,2300,3300",
                ]
            )
            + "\n",
            encoding="utf-8",
        )

        wbe = wb.WbEnvironment()
        wbe.ascii_to_las(
            input_ascii_files=[str(csv_path)],
            pattern="x,y,z,i,c,rn,nr,sa,time,r,g,b",
            epsg_code=4326,
        )

        las_path = tmp_dir / "points.las"
        lidar = wbe.read_lidar(str(las_path))

        cols = ["x", "y", "z", "classification"]
        chunks = lidar.to_numpy_chunks(chunk_size=2, cols=cols)

        for chunk in chunks:
            above_11m = chunk[:, 2] > 11.0
            chunk[above_11m, 3] = 6

        out_path = tmp_dir / "points_reclass_chunked.laz"
        edited = wb.Lidar.from_numpy_chunks(
            chunks,
            base=lidar,
            cols=cols,
            output_path=str(out_path),
        )

        cls = edited.to_numpy(cols=["classification"])
        print("chunked pipeline completed")
        print("classifications:", cls[:, 0].tolist())


if __name__ == "__main__":
    main()
