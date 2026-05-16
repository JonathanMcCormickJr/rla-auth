//! Test-only orchestration for the Dockerized backing services.
//!
//! The unit and integration tests need real Postgres and Redis instances.
//! This module lets the suite be self-contained: the first test to ask for a
//! [`DockerEnv`] brings up the `db` and `redis` services from the repo's
//! `compose.yml`, and once the last [`DockerEnv`] is dropped the services this
//! process started are stopped again. It lives in the library (not behind
//! `#[cfg(test)]`) so the separate integration-test crate can use it too.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant};

/// RAII handle to the running backing services. The services stay up while at
/// least one `DockerEnv` is alive; cloning shares the same handle.
#[derive(Clone)]
pub struct DockerEnv {
    _inner: Arc<DockerEnvInner>,
}

struct DockerEnvInner {
    /// True only when this process ran `docker compose up`. When the services
    /// were already running we leave them as we found them.
    started_by_us: bool,
}

impl Drop for DockerEnvInner {
    fn drop(&mut self) {
        if self.started_by_us {
            let _ = compose(&["stop", "db", "redis"]);
        }
    }
}

static REGISTRY: OnceLock<Mutex<Weak<DockerEnvInner>>> = OnceLock::new();

impl DockerEnv {
    /// Ensure Postgres + Redis are up, starting them if needed. Idempotent and
    /// safe to call concurrently from many tests; blocks until the services
    /// accept connections.
    pub fn ensure() -> DockerEnv {
        let registry = REGISTRY.get_or_init(|| Mutex::new(Weak::new()));
        // Recover from poisoning: if a previous `ensure()` panicked while
        // starting the services, the `Weak` itself is still sound, and the
        // retry should surface the real error rather than a poison panic.
        let mut slot = registry
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if let Some(inner) = slot.upgrade() {
            return DockerEnv { _inner: inner };
        }

        let inner = Arc::new(start_backing_services());
        *slot = Arc::downgrade(&inner);
        DockerEnv { _inner: inner }
    }
}

fn start_backing_services() -> DockerEnvInner {
    if services_ready() {
        return DockerEnvInner {
            started_by_us: false,
        };
    }

    let output = compose(&["up", "-d", "db", "redis"]).expect("failed to spawn `docker compose`");
    assert!(
        output.status.success(),
        "`docker compose up -d db redis` failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        if services_ready() {
            return DockerEnvInner {
                started_by_us: true,
            };
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    panic!("Postgres and Redis did not become ready within 60s");
}

fn services_ready() -> bool {
    postgres_ready() && redis_ready()
}

fn postgres_ready() -> bool {
    compose(&["exec", "-T", "db", "pg_isready", "-U", "postgres"])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn redis_ready() -> bool {
    compose(&["exec", "-T", "redis", "redis-cli", "ping"])
        .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).contains("PONG"))
        .unwrap_or(false)
}

/// Run `docker compose <args>` from the repo root, using the auto-detected
/// Docker invocation and the auth-service `.env` for variable interpolation.
fn compose(args: &[&str]) -> std::io::Result<Output> {
    let (program, prefix) = docker_argv()
        .split_first()
        .expect("docker invocation is never empty");

    Command::new(program)
        .args(prefix)
        .arg("compose")
        .arg("--env-file")
        .arg(env_file())
        .args(args)
        .current_dir(project_root())
        .output()
}

/// The Docker CLI invocation that can actually reach the daemon: plain
/// `docker`, or `sudo -n docker` when the user is not in the `docker` group.
/// Detected once and cached.
fn docker_argv() -> &'static [&'static str] {
    static INVOCATION: OnceLock<Vec<&'static str>> = OnceLock::new();
    INVOCATION.get_or_init(|| {
        if daemon_reachable(&["docker"]) {
            vec!["docker"]
        } else if daemon_reachable(&["sudo", "-n", "docker"]) {
            vec!["sudo", "-n", "docker"]
        } else {
            panic!(
                "Cannot reach the Docker daemon. The test suite starts Postgres \
                 and Redis via Docker. Either add your user to the `docker` group \
                 (`sudo usermod -aG docker $USER`, then re-login) or enable \
                 passwordless sudo for Docker."
            )
        }
    })
}

fn daemon_reachable(argv: &[&str]) -> bool {
    let (program, prefix) = argv.split_first().expect("argv is never empty");
    Command::new(program)
        .args(prefix)
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Repo root: the parent of the auth-service crate, where `compose.yml` lives.
fn project_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("auth-service crate always has a parent directory")
}

fn env_file() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(".env")
}
