import whitebox_workflows
from whitebox_workflows import WbEnvironment
import time
import statistics
import inspect
import os

print(f'File: {whitebox_workflows.__file__}')

wbe = WbEnvironment()
wbe.verbose = False
wbe.working_directory = '/Users/johnlindsay/Documents/data/LakeErieLidar/'

sig = inspect.signature(wbe.rescale_value_range)
print(f'Signature: {sig}')

try:
    dem = wbe.read_raster('LKERIE_10m_final_DEM.tif')
    times = []
    for i in range(5):
        start = time.time()
        res = wbe.rescale_value_range(dem, 0, 255)
        end = time.time()
        times.append(end - start)

    print(f'Run list: {times}')
    print(f'Median: {statistics.median(times)}')
    print(f'Mean: {statistics.mean(times)}')
except Exception as e:
    print(f'Error: {e}')
