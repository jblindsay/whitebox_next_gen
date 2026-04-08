import whitebox_workflows

wbe = whitebox_workflows.WbEnvironment()
# print(type(wbe))
# print(len(wbe.list_tools()))
# print(wbe.search_tools("break line"))


wbe.working_directory = "/Users/johnlindsay/Documents/data/Ponui island NZ/"
dem = wbe.read_raster("DEM.tif")

print(dem.configs().rows)

# wbe.hydrology.fill_depressions_wang_and_liu(
#     dem="D:/data/dem.tif", output="D:/data/dem_filled.tif"
# )