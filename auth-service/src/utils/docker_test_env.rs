//! Test-only orchestration for the backing services (Postgres + Redis).
//!
//! The unit and integration tests need real Postgres and Redis instances.
//! When they are already reachable this module uses them as-is — CI runs them
//! as job service containers, and a developer may simply have them running.
//! Otherwise the first test to ask for a [`DockerEnv`] brings up the `db` and
//! `redis` services from the repo's `compose.yml`, and once the last
//! [`DockerEnv`] is dropped the services this process started are stopped
//! again. It lives in the library (not behind `#[cfg(test)]`) so the separate
//! integration-test crate can use it too.

use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::str::FromStr;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant};

use secrecy::ExposeSecret;
use sqlx::postgres::PgConnectOptions;

use crate::utils::constants::{DATABASE_URL, REDIS_HOST_NAME};

/// Redis listens here unless the connection settings say otherwise.
const DEFAULT_REDIS_PORT: u16 = 6379;

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
    // Fast path: Postgres and Redis are already accepting connections at the
    // addresses the tests use. CI runs them as job service containers, and a
    // developer may have them up already. In both cases we must not touch
    // Docker — `compose.yml`'s `db`/`redis` publish the same host ports, so
    // starting them would collide with the services already bound there.
    if backing_services_reachable() {
        return DockerEnvInner {
            started_by_us: false,
        };
    }

    // Local-developer path: nothing is listening yet, so bring the services up
    // ourselves with `docker compose` and wait for them to accept connections.
    let output = compose(&["up", "-d", "db", "redis"]).expect("failed to spawn `docker compose`");
    assert!(
        output.status.success(),
        "`docker compose up -d db redis` failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        if compose_services_ready() {
            return DockerEnvInner {
                started_by_us: true,
            };
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    panic!("Postgres and Redis did not become ready within 60s");
}

/// Whether Postgres and Redis are already accepting TCP connections at the
/// addresses the tests connect to (parsed from `DATABASE_URL` and
/// `REDIS_HOST_NAME`).
fn backing_services_reachable() -> bool {
    postgres_reachable() && redis_reachable()
}

fn postgres_reachable() -> bool {
    match postgres_host_port() {
        Some((host, port)) => tcp_reachable(&host, port),
        None => false,
    }
}

fn redis_reachable() -> bool {
    tcp_reachable(REDIS_HOST_NAME.as_str(), DEFAULT_REDIS_PORT)
}

/// Host and port Postgres listens on, parsed from `DATABASE_URL`. `get_port`
/// already falls back to the standard 5432 when the URL omits the port.
fn postgres_host_port() -> Option<(String, u16)> {
    let opts = PgConnectOptions::from_str(DATABASE_URL.expose_secret()).ok()?;
    Some((opts.get_host().to_owned(), opts.get_port()))
}

/// Whether any address `host:port` resolves to accepts a TCP connection within
/// a short timeout.
fn tcp_reachable(host: &str, port: u16) -> bool {
    let Ok(mut addrs) = (host, port).to_socket_addrs() else {
        return false;
    };
    addrs.any(|addr| TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok())
}

/// Whether the compose-managed `db` and `redis` containers report ready.
fn compose_services_ready() -> bool {
    compose_postgres_ready() && compose_redis_ready()
}

fn compose_postgres_ready() -> bool {
    compose(&["exec", "-T", "db", "pg_isready", "-U", "postgres"])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn compose_redis_ready() -> bool {
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

    let mut command = Command::new(program);
    command.args(prefix).arg("compose");

    // The auth-service `.env` supplies `compose.yml`'s variable interpolation
    // (e.g. POSTGRES_PASSWORD). It is git-ignored, so it may be absent — only
    // pass `--env-file` when it exists, since Docker errors on a missing one.
    let env_file = env_file();
    if env_file.exists() {
        command.arg("--env-file").arg(env_file);
    }

    command.args(args).current_dir(project_root()).output()
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
