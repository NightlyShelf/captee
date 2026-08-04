# CI Environment Baseline

Captured 2026-08-04 before container-image adoption.

## Target environment and tool inputs

- Target runner: Ubuntu 22.04 x86_64 (`ubuntu-22.04`), Jammy.
- Rust: `1.97.1`, pinned by `rust-toolchain.toml`.
- GTK development package: `libgtk-4-dev` `4.6.9+ds-0ubuntu0.22.04.2`.
- GtkSourceView development package: `libgtksourceview-5-dev` `5.4.1-0ubuntu1`.
- Typst: bundled `0.14.2`, x86_64 musl archive, SHA-256 recorded in
  `tools/typst-version.toml`.
- AppImage packaging tools are currently downloaded from continuous URLs:
  - linuxdeploy: `1-alpha`, commit `07333c6`, build `367`, SHA-256
    `421ca71d5c69ea97c6309276232990d43df1dcece0edfaa26bbf926ff96ed12e`.
  - linuxdeploy GTK plugin: repository `master`, SHA-256
    `b0f4cbc684a0103a9651f0955b635eaea0096b3a66c0f5a2c2aa337960375171`.
  - appimagetool: commit `8c8c91f`, build `295`, SHA-256
    `a6d71e2b6cd66f8e8d16c37ad164658985e0cf5fcaa950c90a482890cb9d13e0`.
  - Type-2 runtime: continuous `runtime-x86_64`, SHA-256
    `1cc49bcf1e2ccd593c379adb17c9f85a36d619088296504de95b1d06215aebbf`.

The local workstation is Arch Linux with Rust `1.97.1`, GTK `4.22.4`, and
GtkSourceView `5.20.0`; Docker is installed but the current user cannot access
the Docker socket, so local image build timings are not available yet.

## Existing workflow timings

Measurements come from successful GitHub Actions runs on `main`:

| Workflow/run | Total | Setup observations |
| --- | ---: | --- |
| Rust checks, [run 30933669778](https://github.com/NightlyShelf/captee/actions/runs/30933669778) | 48 s | Parallel jobs; GTK install 18–24 s, Rust toolchain 9–10 s, Cargo cache 2–4 s. The test/lint commands took 2–3 s after setup. |
| Manual AppImage, [run 30932010190](https://github.com/NightlyShelf/captee/actions/runs/30932010190) | 110 s | GTK/AppImage package install 28 s, Rust toolchain 9 s, Cargo cache 5 s, packaging-tool fetch 1 s, AppImage build 49 s, artifact upload 3 s. |

### Outlier and bottleneck

The first attempt of Rust checks on [run 30932946612](https://github.com/NightlyShelf/captee/actions/runs/30932946612)
was not a normal successful build: the `clippy` job remained in GTK package
installation for 7m03s (7m18s including job setup) and was cancelled. Its
rerun completed successfully in about 1m34s. This is the primary setup
bottleneck this change targets. The current evidence does not show a 7-minute
Cargo or AppImage compilation step in the recorded successful runs; the long
process was the repeated package installation, which also blocks compilation
from starting.

The baseline does not include a container pull because the current workflows do
not use container jobs. Future comparisons must report image pull time,
registry cache hit/miss state, and total job time separately.
