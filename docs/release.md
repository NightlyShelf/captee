# Runtime, recovery, and troubleshooting

## Supported Linux sessions

The first release targets x86_64 Linux with GTK 4.22.4, GLib, and a working
Wayland or X11 desktop session. Project editing and preview do not require a
capture portal, but screenshot capture requires a compositor-supported desktop
portal or the configured `grim`/`slurp` fallback commands.

The GTK development packages are needed to build Captee, not to run the
finished AppImage. The AppImage bundles the application-side GTK libraries;
the host still supplies the display server, compositor, fonts, portals, and
graphics drivers.

The AppImage packaging path has been exercised through AppDir creation. A
final artifact still requires the Type-2 `runtime-x86_64` file supplied to
`APPIMAGE_RUNTIME_FILE`; if appimagetool cannot download it, the build stops
before emitting an artifact rather than producing an incomplete image.

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
