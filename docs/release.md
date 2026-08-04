# Runtime, recovery, and troubleshooting

## Supported Linux sessions

The first release targets x86_64 Linux with a bundled GTK runtime, GLib, and a
working Wayland or X11 desktop session. Local development is validated against
GTK 4.22.4; the Ubuntu 22.04 AppImage builder uses its available GTK 4.6.9
development packages and the application stays within the compatible GTK 4.0
API subset. Project editing and preview do not require a capture portal, but
screenshot capture requires a compositor-supported desktop portal or the
configured `grim`/`slurp` fallback commands.

The GTK development packages are needed to build Captee, not to run the
finished AppImage. The AppImage bundles the application-side GTK libraries;
the host still supplies the display server, compositor, fonts, portals, and
graphics drivers.

The AppImage packaging path has been verified by the manual Ubuntu 22.04
workflow. The successful run is
[`30927742654`](https://github.com/NightlyShelf/captee/actions/runs/30927742654)
at commit `d978b19c1d9033fdb2888f3cd3b978f677e719eb`, and its uploaded artifact
is `captee-appimage-x86_64` (artifact ID `8899853272`). The local handoff copy
is `dist/appimage/Captee-x86_64.AppImage` with SHA-256
`cff84148cf54bfdf45c782778655764f5074331c32b0cbbb1de5bd02a65e6c3d`.

The workflow supplies the Type-2 `runtime-x86_64` file explicitly through
`APPIMAGE_RUNTIME_FILE`; if a packaging tool or runtime cannot be fetched, the
build stops before emitting an incomplete image.

## Reproducible packaging inputs

The successful Ubuntu 22.04 x86_64 run used Rust 1.97.1, GTK/GtkSourceView
development packages from the runner, and these downloaded tool inputs:

| Input | Source/version | SHA-256 |
| --- | --- | --- |
| linuxdeploy | continuous, git `07333c6`, build 367 | `421ca71d5c69ea97c6309276232990d43df1dcece0edfa26bbf926ff96ed12e` |
| linuxdeploy GTK plugin | `master` at workflow run time | `b0f4cbc684a0103a9651f0955b635eaea0096b3a66c0f5a2c2aa337960375171` |
| appimagetool | continuous, git `8c8c91f`, build 295 | `a6d71e2b6cd66f8e8d16c37ad164658985e0cf5fcaa950c90a482890cb9d13e0` |
| Type-2 runtime | `runtime-x86_64`, continuous | `1cc49bcf1e2ccd593c379adb17c9f85a36d619088296504de95b1d06215aebbf` |

Typst 0.14.2 is fetched and checked separately using the pinned archive and
digest in [docs/third-party-typst.md](third-party-typst.md).

## Project layout and recovery

A project contains a versioned `captee.json`, the configured `.typ` entry
document, and confirmed screenshots under `img/`. Writes use a same-directory
temporary file followed by an atomic replacement. Autosaves use a separate
revision-marked file and are never silently substituted for the user document.

On startup, if an autosave is newer than the entry document, copy it to a safe
location or explicitly accept recovery after comparing the revision and source
contents. A failed save leaves the last complete destination in place and
cleans its temporary file.

Capture cancellation happens before asset persistence and editor insertion, so
cancelled captures do not create an image or change the Typst source. Confirmed
assets are validated PNGs with bounded dimensions and are stored only below the
project `img/` directory.

## Capture permissions

The portal backend is attempted first. A portal cancellation is a deliberate
no-op; fallback capture is attempted only after a portal failure and when the
fallback setting is enabled. On Wayland, grant the desktop portal permission
when prompted. On X11 or minimal compositors, install and test `grim` and
`slurp` if the fallback is required.

## Bundled Typst and licensing

The release bundles Typst 0.14.2 from the pinned upstream archive. The archive
digest, source URL, and retained upstream `LICENSE`/`NOTICE` files are recorded
in [docs/third-party-typst.md](third-party-typst.md). Do not replace the
compiler binary without updating that manifest and the preview/diagnostic
fixtures.

## Troubleshooting

- **The UI does not start:** verify a Wayland or X11 session, `GTK 4.22.4` runtime libraries, and graphics-driver availability. Use `G_MESSAGES_DEBUG=all cargo run -p captee-ui` for GTK diagnostics.
- **Capture is unavailable:** verify the portal service and permission prompt; for fallback capture, run `command -v grim slurp` and test the commands independently.
- **Preview fails:** inspect the diagnostic panel and confirm the entry document and referenced assets stay inside the project. The last successful preview remains available after a failed render.
- **An asset is rejected:** confirm it is a complete PNG and below the documented pixel/byte limits. Invalid or missing `img/` directories are rejected before mutation.
- **Recovery is offered unexpectedly:** compare the autosave revision and timestamp with the entry document before accepting it; never overwrite the original until the recovered copy is reviewed.
