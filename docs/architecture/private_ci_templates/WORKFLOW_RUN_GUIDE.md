# Private Pro Workflow Run Guide

This guide explains the difference between local smoke tests and real GitHub Actions workflow runs.

## Key point

- GitHub workflow `.yml` files run natively on GitHub Actions runners.
- Local execution should validate the same build commands, not attempt full runner parity.

## Where real workflow runs happen

Run these in the private repository Actions tab:

- Build wbw_python Pro Wheels: `.github/workflows/wbw-python-pro-build.yml`
- Build wbw_r Open + Pro Artifacts: `.github/workflows/wbw-r-dual-build.yml`

## Recommended workflow-dispatch sequence

1. Open Actions in private repo.
2. Run `Build wbw_python Pro Wheels` with `whitebox_next_gen_ref=main`.
3. Confirm artifacts upload successfully.
4. Run `Build wbw_r Open + Pro Artifacts` with `whitebox_next_gen_ref=main`.
5. Confirm both open and pro artifacts upload successfully.

## Local smoke test (command parity)

Use this only as a preflight check before GitHub Actions.

### 1) Verify private overlay script presence

```bash
cd /Users/johnlindsay/Documents/programming/Rust/wbtools_pro
ls -la ci/apply_pro_overlay.sh
```

### 2) Verify private workflows call overlay script

```bash
cd /Users/johnlindsay/Documents/programming/Rust/wbtools_pro
grep -n "Apply private Pro overlay" .github/workflows/wbw-python-pro-build.yml
grep -n "Apply private Pro overlay" .github/workflows/wbw-r-dual-build.yml
```

### 3) Verify public repo points bindings at shim

```bash
cd /Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen
grep -n "wbtools_pro" crates/wbw_python/Cargo.toml
grep -n "wbtools_pro" crates/wbw_r/Cargo.toml
```

### 4) Verify shim contract and public buildability checks

```bash
cd /Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen
cargo test -p wbtools_pro --test contract
cargo check -p wbw_python -p wbw_r
cargo check -p wbw_python --features pro
cargo check -p wbw_r --features pro
```

## Notes

- Do not treat local smoke tests as equivalent to GitHub Actions matrix runs.
- Artifact packaging and cross-OS behavior are validated only in GitHub Actions.
