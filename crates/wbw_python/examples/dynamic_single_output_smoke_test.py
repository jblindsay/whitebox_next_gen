import whitebox_workflows as wb


def main() -> None:
    wbe = wb.WbEnvironment(include_pro=True, tier="pro")
    wbe.working_directory = "/Users/johnlindsay/Documents/data/UofWaterloo_lidar/"

    dem = wbe.read_raster("DEM.tif")

    out_plain = wbe.raster.abs(input=dem)
    assert hasattr(out_plain, "file_path"), type(out_plain)
    assert not isinstance(out_plain, (list, tuple)), type(out_plain)

    def callback(_event):
        return None

    out_callback = wbe.raster.abs(input=dem, callback=callback)
    assert hasattr(out_callback, "file_path"), type(out_callback)
    assert not isinstance(out_callback, (list, tuple)), type(out_callback)

    assert str(out_plain.file_path).startswith("memory://raster/")
    assert str(out_callback.file_path).startswith("memory://raster/")

    print("dynamic single-output smoke test passed")


if __name__ == "__main__":
    main()
