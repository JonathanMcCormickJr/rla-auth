## Prerequisites

- `auth-service/.env` must exist with at least:
  ```
  POSTGRES_PASSWORD=<password>
  JWT_SECRET=<secret>
  DATABASE_URL=postgres://postgres:<password>@localhost:5432
  ```
- Docker must be installed and the current user must have permission to run it.

## Setup & Building
```bash
cargo install cargo-watch
cd app-service
cargo build
cd ..
cd auth-service
cargo build
cd ..
```

## Backing services (Postgres + Redis)

Postgres and Redis run as standalone Docker containers (`ps-db` on host port 5432, `redis-db` on host port 6379), managed by the reset scripts in `auth-service/scripts/`. They are used by **both** the local `cargo run` workflow and the Docker Compose workflow.

`docker.sh` invokes `reset_all.sh` automatically, so for the Compose workflow you usually don't need to call the reset scripts directly. Run them manually when you want fresh state for local `cargo run`, or when only one port is stuck:

```bash
sudo ./auth-service/scripts/reset_all.sh             # reset both
sudo ./auth-service/scripts/reset-docker-db.sh       # reset only Postgres
sudo ./auth-service/scripts/reset-docker-redis.sh    # reset only Redis
```

These scripts intentionally *do not* go through Docker Compose — they own the `ps-db` and `redis-db` container lifecycle so Compose does not fight them for ports 5432/6379.

## Run servers locally (manually)

Bring up Postgres and Redis first:
```bash
sudo ./auth-service/scripts/reset_all.sh
```

#### App service
```bash
cd app-service
cargo watch -q -c -w src/ -w assets/ -w templates/ -x run
```

visit http://localhost:8000

#### Auth service
```bash
cd auth-service
cargo watch -q -c -w src/ -w assets/ -x run
```

visit http://localhost:3000

## Run servers locally (Docker Compose)

```bash
sudo ./docker.sh
```

That single command (a) loads `auth-service/.env`, (b) resets the standalone Postgres + Redis containers via `reset_all.sh`, and (c) builds and starts `auth-service` + `app-service` via `docker compose`. The Compose file only defines `auth-service` and `app-service`; they reach the standalone `ps-db` / `redis-db` containers via `host.docker.internal` (wired with `extra_hosts: host-gateway` for Linux).

visit http://localhost:8000 and http://localhost:3000

Also, try for 147.182.214.35:8000 and 147.182.214.35:3000

