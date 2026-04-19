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

Postgres and Redis are defined as Docker Compose services (`db` on host port 5432, `redis` on host port 6379) in `compose.yml`. The reset scripts in `auth-service/scripts/` are thin wrappers around `docker compose` that tear down a service (including its volume) and start it again with fresh state:

```bash
sudo ./auth-service/scripts/reset_all.sh             # reset both
sudo ./auth-service/scripts/reset-docker-db.sh       # reset only Postgres
sudo ./auth-service/scripts/reset-docker-redis.sh    # reset only Redis
```

`docker.sh` invokes `reset_all.sh` automatically, so for the Compose workflow you usually don't need to call the reset scripts directly. Reach for them when you want a clean DB for local `cargo run`, or when state has gotten wedged.

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

That single command (a) loads `auth-service/.env`, (b) resets the Postgres + Redis compose services via `reset_all.sh`, and (c) builds and starts `auth-service` + `app-service`. Inside the compose network auth-service reaches Postgres at `db:5432` and Redis at `redis:6379` (the auth-service Dockerfile sets `REDIS_HOST_NAME=redis`).

visit http://localhost:8000 and http://localhost:3000

Also, try for 147.182.214.35:8000 and 147.182.214.35:3000

