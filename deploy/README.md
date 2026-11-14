# Deployment assets

Planned location for the reference deployment. **No deployment assets exist yet.**

Expected files:

```text
deploy/
  Dockerfile
  docker-compose.yml
  .env.example
  caddy/Caddyfile
  nginx/weft.conf
  README.md
```

Until the MVP exists, use [`../docs/OPERATIONS.md`](../docs/OPERATIONS.md) as the
target procedure. Do not claim that a Compose stack works before it has been
tested from a clean checkout.

Rules:

- Pin image versions.
- Do not include secrets in the repository.
- Keep the application behind a TLS-terminating reverse proxy.
- Mount the blob volume and PostgreSQL data as persistent volumes.
- Expose only the reverse proxy to the internet; bind PostgreSQL and the app to
  localhost by default.
