from __future__ import annotations

from pathlib import Path

import whitebox_workflows as wb


DATA_ROOT = Path('/Users/johnlindsay/Documents/data')


def main() -> None:
    wbe = wb.WbEnvironment()
    vector_path = DATA_ROOT / 'Ottawa DEM' / 'Ottawa_soils_data.shp'
    vector = wbe.read_vector(str(vector_path))

    print('Schema fields (first 10):')
    for fld in vector.schema()[:10]:
        print('  ', fld)

    if vector.feature_count() == 0:
        print('No features to inspect.')
        return

    attrs0 = vector.attributes(0)
    print('Feature 0 keys:', list(attrs0.keys())[:10])

    if 'SiteID' in attrs0:
        print('Feature 0 SiteID:', vector.attribute(0, 'SiteID'))


if __name__ == '__main__':
    main()
