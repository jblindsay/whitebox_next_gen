from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

import whitebox_workflows as wb


DEFAULT_OUTPUT_DIR = Path(__file__).resolve().parent / 'output' / 'sensor_bundles'

FAMILY_LOADERS: list[tuple[str, str, str]] = [
    ('bundle', 'read_bundle', 'Auto-detect any supported bundle family.'),
    ('landsat', 'read_landsat', 'Open a Landsat bundle.'),
    ('sentinel1', 'read_sentinel1', 'Open a Sentinel-1 SAFE bundle.'),
    ('sentinel2', 'read_sentinel2', 'Open a Sentinel-2 SAFE bundle.'),
    ('planetscope', 'read_planetscope', 'Open a PlanetScope bundle.'),
    ('iceye', 'read_iceye', 'Open an ICEYE bundle.'),
    ('dimap', 'read_dimap', 'Open a DIMAP bundle.'),
    ('maxar-worldview', 'read_maxar_worldview', 'Open a Maxar/WorldView bundle.'),
    ('radarsat2', 'read_radarsat2', 'Open a RADARSAT-2 bundle.'),
    ('rcm', 'read_rcm', 'Open an RCM bundle.'),
]

@dataclass(frozen=True)
class BundleJob:
    loader_name: str
    cli_label: str
    bundle_root: Path


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description='Inspect and manipulate supported remote sensing sensor bundle types.',
        epilog=(
            'Examples:\n'
            '  python examples/sensor_bundle_overview.py '
            '--sentinel2 /data/S2A_MSIL2A_...SAFE\n'
            '  python examples/sensor_bundle_overview.py '
            '--sentinel1 /data/S1A_IW_GRDH_...SAFE --landsat /data/LC09_L2SP_...\n'
            '  python examples/sensor_bundle_overview.py '
            '--bundle /data/unknown_bundle_root --output-dir examples/output/sensor_bundles'
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    for cli_name, _loader_name, help_text in FAMILY_LOADERS:
        parser.add_argument(
            f'--{cli_name}',
            dest=cli_name.replace('-', '_'),
            action='append',
            default=[],
            metavar='PATH',
            help=help_text,
        )
    parser.add_argument(
        '--output-dir',
        type=Path,
        default=DEFAULT_OUTPUT_DIR,
        help=(
            'Directory for preview outputs. '
            f'Default: {DEFAULT_OUTPUT_DIR}'
        ),
    )
    parser.add_argument(
        '--no-output',
        action='store_true',
        help='Inspect bundles without writing preview rasters.',
    )
    return parser


def collect_jobs(args: argparse.Namespace) -> list[BundleJob]:
    jobs: list[BundleJob] = []
    for cli_name, loader_name, _help_text in FAMILY_LOADERS:
        for raw_path in getattr(args, cli_name.replace('-', '_')):
            jobs.append(BundleJob(loader_name=loader_name, cli_label=cli_name, bundle_root=Path(raw_path)))
    return jobs


def print_optional(label: str, value: object) -> None:
    if value is None:
        return
    if isinstance(value, list) and not value:
        return
    print(f'  {label}: {value}')


def preview_list(values: Iterable[str], limit: int = 8) -> str:
    items = list(values)
    if not items:
        return '[]'
    if len(items) <= limit:
        return str(items)
    return f'{items[:limit]} ... (+{len(items) - limit} more)'


def sanitize_fragment(value: str) -> str:
    sanitized = []
    for ch in value:
        if ch.isalnum() or ch in ('-', '_'):
            sanitized.append(ch)
        else:
            sanitized.append('_')
    return ''.join(sanitized).strip('_') or 'item'


def read_preview_raster(bundle: wb.Bundle) -> tuple[str, str, wb.Raster] | None:
    band_keys = bundle.list_band_keys()
    if band_keys:
        key = band_keys[0]
        return 'band', key, bundle.read_band(key)

    measurement_keys = bundle.list_measurement_keys()
    if measurement_keys:
        key = measurement_keys[0]
        return 'measurement', key, bundle.read_measurement(key)

    asset_keys = bundle.list_asset_keys()
    if asset_keys:
        key = asset_keys[0]
        return 'asset', key, bundle.read_asset(key)

    qa_keys = bundle.list_qa_keys()
    if qa_keys:
        key = qa_keys[0]
        return 'qa', key, bundle.read_qa_layer(key)

    aux_keys = bundle.list_aux_keys()
    if aux_keys:
        key = aux_keys[0]
        return 'aux', key, bundle.read_aux_layer(key)

    return None


def try_write_rgb_preview(wbe: wb.WbEnvironment, bundle: wb.Bundle, output_dir: Path, stem: str) -> tuple[Path | None, Path | None]:
    """Attempt to write true-colour and false-colour previews using the new bundle helpers."""
    tcc_path: Path | None = None
    fcc_path: Path | None = None
    try:
        tcc_output = output_dir / f'{stem}_true_colour.tif'
        wbe.true_colour_composite(bundle.bundle_root, output_path=str(tcc_output))
        tcc_path = tcc_output
    except Exception:
        pass  # SAR families and families with missing bands are skipped gracefully
    try:
        fcc_output = output_dir / f'{stem}_false_colour.tif'
        wbe.false_colour_composite(bundle.bundle_root, output_path=str(fcc_output))
        fcc_path = fcc_output
    except Exception:
        pass
    return tcc_path, fcc_path


def inspect_bundle(wbe: wb.WbEnvironment, job: BundleJob, output_dir: Path | None) -> None:
    loader = getattr(wbe, job.loader_name)
    bundle = loader(str(job.bundle_root))

    print(f'\n[{job.cli_label}] {job.bundle_root}')
    print_optional('family', bundle.family)
    print_optional('bundle_root', bundle.bundle_root)
    print_optional('acquired_utc', bundle.acquisition_datetime_utc())
    print_optional('processing_level', bundle.processing_level())
    print_optional('tile_id', bundle.tile_id())
    print_optional('mission', bundle.mission())
    print_optional('product_type', bundle.product_type())
    print_optional('acquisition_mode', bundle.acquisition_mode())
    print_optional('cloud_cover_percent', bundle.cloud_cover_percent())
    print_optional('polarizations', bundle.polarizations())

    print_optional('band_keys', preview_list(bundle.list_band_keys()))
    print_optional('measurement_keys', preview_list(bundle.list_measurement_keys()))
    print_optional('qa_keys', preview_list(bundle.list_qa_keys()))
    print_optional('aux_keys', preview_list(bundle.list_aux_keys()))
    print_optional('asset_keys', preview_list(bundle.list_asset_keys()))

    preview = read_preview_raster(bundle)
    if preview is None:
        print('  preview: no raster-like assets exposed for this bundle')
        return

    preview_kind, preview_key, preview_raster = preview
    preview_meta = preview_raster.metadata()
    print(
        '  preview_raster: '
        f"{preview_kind} '{preview_key}' -> {preview_meta.rows} x {preview_meta.columns}, "
        f'nodata={preview_meta.nodata}'
    )

    if output_dir is None:
        return

    output_dir.mkdir(parents=True, exist_ok=True)
    stem = sanitize_fragment(job.bundle_root.stem or job.bundle_root.name)
    eq_output = output_dir / f'{stem}_{preview_kind}_{sanitize_fragment(preview_key)}_equalized.tif'
    equalized = wbe.histogram_equalization(preview_raster)
    wbe.write_raster(equalized, str(eq_output))
    print(f'  wrote_equalized_preview: {eq_output}')

    tcc_output, fcc_output = try_write_rgb_preview(wbe, bundle, output_dir, stem)
    if tcc_output is not None:
        print(f'  wrote_true_colour: {tcc_output}')
    if fcc_output is not None:
        print(f'  wrote_false_colour: {fcc_output}')


def main() -> None:
    parser = build_parser()
    args = parser.parse_args()
    jobs = collect_jobs(args)

    if not jobs:
        parser.error('Provide at least one bundle path, e.g. --sentinel2 PATH or --bundle PATH.')

    output_dir = None if args.no_output else args.output_dir

    wbe = wb.WbEnvironment()
    for job in jobs:
        inspect_bundle(wbe, job, output_dir)


if __name__ == '__main__':
    main()