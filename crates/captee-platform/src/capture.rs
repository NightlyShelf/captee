//! Portal-first capture selection and bounded `grim`/`slurp` fallback.

use captee_core::{CaptureBackend, CaptureError, CaptureResult, CaptureSettings, CapturedImage};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Selects the configured portal backend first and falls back only after a
/// portal failure. Cancellation is returned immediately and never falls back.
#[derive(Debug, Clone)]
pub struct CaptureSelector<P, F> {
    portal: P,
    fallback: F,
    settings: CaptureSettings,
}

impl<P, F> CaptureSelector<P, F> {
    pub fn new(portal: P, fallback: F, settings: CaptureSettings) -> Self {
        Self { portal, fallback, settings }
    }
}

impl<P: CaptureBackend, F: CaptureBackend> CaptureBackend for CaptureSelector<P, F> {
    fn capture(&self) -> CaptureResult<CapturedImage> {
        if self.settings.portal_enabled {
            match self.portal.capture() {
                CaptureResult::Completed(image) => return CaptureResult::Completed(image),
                CaptureResult::Cancelled => return CaptureResult::Cancelled,
                CaptureResult::Failed(error) if !self.settings.fallback_enabled => {
                    return CaptureResult::Failed(error)
                }
                CaptureResult::Failed(_) => {}
            }
        }

        if self.settings.fallback_enabled {
            return self.fallback.capture();
        }

        CaptureResult::Failed(CaptureError::new("no capture backend is enabled"))
    }
}

/// Bounded Linux fallback using `slurp` for region selection and `grim` for
/// PNG capture. The executable paths are configurable for packaging and tests.
#[derive(Debug, Clone)]
pub struct GrimSlurpCapture {
    slurp: PathBuf,
    grim: PathBuf,
    timeout: Duration,
}

impl GrimSlurpCapture {
    pub fn new(timeout: Duration) -> Self {
        Self::with_paths("slurp", "grim", timeout)
    }

    pub fn with_paths(
        slurp: impl Into<PathBuf>,
        grim: impl Into<PathBuf>,
        timeout: Duration,
    ) -> Self {
        Self { slurp: slurp.into(), grim: grim.into(), timeout }
    }
}

impl CaptureBackend for GrimSlurpCapture {
    fn capture(&self) -> CaptureResult<CapturedImage> {
        let selection = match run_bounded(&self.slurp, &[], self.timeout) {
            Ok(output) => output,
            Err(error) => return CaptureResult::Failed(error),
        };
        if !selection.status.success() {
            if selection.status.code() == Some(1) {
                return CaptureResult::Cancelled;
            }
            return CaptureResult::Failed(command_failure("slurp", &selection));
        }

        let geometry = String::from_utf8_lossy(&selection.stdout).trim().to_owned();
        if geometry.is_empty() {
            return CaptureResult::Cancelled;
        }

        let grim_args = vec!["-g".to_owned(), geometry, "-".to_owned()];
        let image = match run_bounded(&self.grim, &grim_args, self.timeout) {
            Ok(output) => output,
            Err(error) => return CaptureResult::Failed(error),
        };
        if !image.status.success() {
            return CaptureResult::Failed(command_failure("grim", &image));
        }
        CaptureResult::Completed(CapturedImage::new(image.stdout))
    }
}

fn run_bounded(program: &Path, args: &[String], timeout: Duration) -> Result<Output, CaptureError> {
    let mut command = Command::new(program);
    command.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| {
        CaptureError::new(format!("could not start {}: {error}", program.display()))
    })?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child.wait_with_output().map_err(|error| {
                    CaptureError::new(format!("could not read capture output: {error}"))
                })
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(CaptureError::new(format!(
                    "capture command timed out after {} ms",
                    timeout.as_millis()
                )));
            }
            Ok(None) => thread::sleep(Duration::from_millis(5)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(CaptureError::new(format!(
                    "could not monitor capture command: {error}"
                )));
            }
        }
    }
}

fn command_failure(command: &str, output: &Output) -> CaptureError {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if stderr.is_empty() {
        CaptureError::new(format!("{command} exited with status {}", output.status))
    } else {
        CaptureError::new(format!("{command} failed: {stderr}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct FakeBackend {
        result: CaptureResult<CapturedImage>,
        calls: Arc<Mutex<u32>>,
    }

    impl CaptureBackend for FakeBackend {
        fn capture(&self) -> CaptureResult<CapturedImage> {
            *self.calls.lock().expect("calls lock") += 1;
            self.result.clone()
        }
    }

    fn backend(result: CaptureResult<CapturedImage>) -> (FakeBackend, Arc<Mutex<u32>>) {
        let calls = Arc::new(Mutex::new(0));
        (FakeBackend { result, calls: calls.clone() }, calls)
    }

    fn test_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos();
        let root = std::env::temp_dir().join(format!("captee-capture-{name}-{suffix}"));
        fs::create_dir_all(&root).expect("temporary root");
        root
    }

    #[test]
    fn portal_success_is_preferred_over_fallback() {
        let (portal, portal_calls) =
            backend(CaptureResult::Completed(CapturedImage::new(b"portal")));
        let (fallback, fallback_calls) =
            backend(CaptureResult::Completed(CapturedImage::new(b"fallback")));
        let selector = CaptureSelector::new(portal, fallback, CaptureSettings::default());

        assert_eq!(selector.capture(), CaptureResult::Completed(CapturedImage::new(b"portal")));
        assert_eq!(*portal_calls.lock().expect("portal calls"), 1);
        assert_eq!(*fallback_calls.lock().expect("fallback calls"), 0);
    }

    #[test]
    fn portal_failure_uses_enabled_fallback_but_cancellation_does_not() {
        let (portal, _) = backend(CaptureResult::Failed(CaptureError::new("portal unavailable")));
        let (fallback, fallback_calls) =
            backend(CaptureResult::Completed(CapturedImage::new(b"fallback")));
        let selector = CaptureSelector::new(portal, fallback, CaptureSettings::default());
        assert_eq!(selector.capture(), CaptureResult::Completed(CapturedImage::new(b"fallback")));
        assert_eq!(*fallback_calls.lock().expect("fallback calls"), 1);

        let (portal, _) = backend(CaptureResult::Cancelled);
        let (fallback, fallback_calls) =
            backend(CaptureResult::Completed(CapturedImage::new(b"fallback")));
        let selector = CaptureSelector::new(portal, fallback, CaptureSettings::default());
        assert_eq!(selector.capture(), CaptureResult::Cancelled);
        assert_eq!(*fallback_calls.lock().expect("fallback calls"), 0);
    }

    #[test]
    fn disabled_backends_return_an_explicit_failure() {
        let (portal, portal_calls) =
            backend(CaptureResult::Completed(CapturedImage::new(b"portal")));
        let (fallback, fallback_calls) =
            backend(CaptureResult::Completed(CapturedImage::new(b"fallback")));
        let selector = CaptureSelector::new(
            portal,
            fallback,
            CaptureSettings { portal_enabled: false, fallback_enabled: false },
        );

        assert!(matches!(selector.capture(), CaptureResult::Failed(_)));
        assert_eq!(*portal_calls.lock().expect("portal calls"), 0);
        assert_eq!(*fallback_calls.lock().expect("fallback calls"), 0);
    }

    #[cfg(unix)]
    #[test]
    fn grim_slurp_adapter_runs_selection_then_capture_with_a_bound() {
        use std::os::unix::fs::PermissionsExt;

        let root = test_root("commands");
        let slurp = root.join("slurp");
        let grim = root.join("grim");
        fs::write(&slurp, "#!/bin/sh\nprintf '0,0 10x10'\n").expect("slurp script");
        fs::write(&grim, "#!/bin/sh\nprintf 'PNG fixture'\n").expect("grim script");
        for path in [&slurp, &grim] {
            let mut permissions = fs::metadata(path).expect("script metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).expect("script permissions");
        }

        let capture = GrimSlurpCapture::with_paths(slurp, grim, Duration::from_secs(1));
        assert_eq!(capture.capture(), CaptureResult::Completed(CapturedImage::new(b"PNG fixture")));
        fs::remove_dir_all(root).expect("cleanup");
    }
}
