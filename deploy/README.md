# Deployment assets

Milestone 0 deployment profile. It builds the `weft-cli` binary and runs it
against PostgreSQL.

```text
deploy/
  Dockerfile
  docker-compose.yml
  config.toml
  .env.example
  README.md
```

Start it with:

```bash
cp deploy/.env.example deploy/.env
docker compose -f deploy/docker-compose.yml up -d --build
curl http://127.0.0.1:8080/healthz
```

The Compose profile is a development/reference deployment. It has not been
tested against a production reverse proxy or public traffic. Use
[`../docs/OPERATIONS.md`](../docs/OPERATIONS.md) for the operational procedure.

Rules:

- Pin image versions.
- Do not include secrets in the repository.
- Keep the application behind a TLS-terminating reverse proxy.
- Mount the blob volume and PostgreSQL data as persistent volumes.
- Expose only the reverse proxy to the internet; bind PostgreSQL and the app to
  localhost by default.
