import whitebox_workflows as wb

import whitebox_workflows as wb

wbe = wb.WbEnvironment(include_pro=True, tier='pro')
print('yield_data_conditioning_and_qa' in set(wbe.list_tools()))

# wbe.vector_tools

# wbe = wb.WbEnvironment()
# print(type(wbe))
# print(len(wbe.list_tools()))
# print(wbe.search_tools("break line"))


# wbe.working_directory = "/Users/johnlindsay/Documents/data/Ponui island NZ/"
# dem = wbe.read_raster("DEM.tif")
# dem_meta = dem.metadata()
# print(dem_meta.rows)

# matches = wbe.search_tools("sar")
# print(matches)

# wbe.hydrology.fill_depressions_wang_and_liu(
#     dem="D:/data/dem.tif", output="D:/data/dem_filled.tif"
# )