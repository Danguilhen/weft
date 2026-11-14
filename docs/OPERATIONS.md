# Weft — operations and self-hosting

Status: design. Commands and file names become authoritative when the MVP is
implemented; update this document with the code.

## 1. Minimum node

The recommended public small node:

- 2 vCPU, 2 GB RAM, 20 GB storage;
- PostgreSQL 16;
- a reverse proxy with TLS;
- outbound HTTPS for federation and replication (optional in the MVP).

A 1 vCPU / 1 GB / 10 GB node is possible for a small, read-mostly library with the
conservative decode limits in `CONFIG.md` (128 MB, 8 MP, 150 frames). Measure
before recommending it for public uploads.

A node with federation and replication disabled needs no inbound peer access
beyond the public HTTP API.

## 2. Quick start (target)

```bash
git clone <repository> weft
cd weft
cp deploy/.env.example deploy/.env
# edit deploy/.env
docker compose -f deploy/docker-compose.yml up -d
docker compose -f deploy/docker-compose.yml exec weft weft init
docker compose -f deploy/docker-compose.yml exec weft weft migrate
```

The node listens on `127.0.0.1:8080`; the reverse proxy publishes HTTPS.

## 3. Reverse proxy

The application does not terminate TLS. Example Caddy:

```caddy
gifs.example {
    encode zstd gzip
    reverse_proxy 127.0.0.1:8080
    header {
        Strict-Transport-Security "max-age=31536000; includeSubDomains"
        X-Content-Type-Options "nosniff"
        Referrer-Policy "no-referrer"
    }
    request_body {
        max_size 20MB
    }
}
```

Nginx and Traefik examples belong in `deploy/` once tested.

Set `server.trusted_proxies` so forwarded addresses are only trusted from the
proxy. Do not trust `X-Forwarded-For` from the open internet.

## 4. Configuration

See [`CONFIG.md`](CONFIG.md). Keep `config.toml` at `0640` and secrets in the
environment or a root-only secret file. `weft init` must refuse to overwrite an
existing key.

## 5. Backups

Back up:

- the PostgreSQL database;
- the blob and thumbnail volumes;
- the node key and hash salt (separately, encrypted);
- the configuration with secrets removed or encrypted.

Example:

```bash
pg_dump --format=custom --file=weft-$(date +%F).dump "$DATABASE_URL"
tar --zstd -cf blobs-$(date +%F).tar.zst /var/lib/weft
```

Do not back up quarantined illegal content unless legally required. Document the
retention and destruction of backups.

### Restore drill

1. Restore PostgreSQL into an empty database.
2. Restore the blob volume.
3. Restore the key and salt with the original permissions.
4. Run `weft migrate` and `weft verify --all`.
5. Fetch a sample hash and compare the digest.
6. Check that removed content is still tombstoned.

Run this drill before the first public release and after schema changes.

## 6. Upgrades

- Read the changelog and the migration notes.
- Back up the database and blobs.
- Pull the new image, stop the service, run migrations, start the service.
- Verify health, a sample manifest and a sample media fetch.
- Keep the previous image until the new one is verified.

Migrations are forward-only. A rollback requires restoring the backup; do not
edit an applied migration.

## 7. Monitoring

Expose:

- `GET /healthz` — process alive;
- `GET /readyz` — database and storage ready;
- `GET /metrics` — Prometheus format.

Alert on:

- disk usage > 80%;
- PostgreSQL connection failures;
- decode timeouts or validation failures spiking;
- 5xx rate;
- orphan blobs;
- replication queue depth (Phase 3);
- expired keys or certificates.

Do not put upload bodies, tokens or client addresses in metrics labels.

## 8. Capacity

- Blobs dominate storage; thumbnails are small.
- PostgreSQL grows with metadata and events, not media.
- Plan egress for popular content; replication is opt-in.
- The MVP does not transcode, so CPU is mostly decode and TLS.
- Keep at least 20% free disk for temporary files and PostgreSQL WAL.

## 9. Keys

- Node private key: root-only file or secret manager; never in the database or
  logs.
- Rotate on a schedule or on suspicion.
- Publish the new public key in the actor and descriptor before retiring the old
  one.
- Keep old public keys for verification history; mark `validUntil`/`revokedAt`.
- Rotating the key does not change the actor URL or the node identity.

## 10. Moderation operations

- Review the quarantine queue daily on a public node.
- Keep a written takedown and illegal-content procedure.
- Record decisions as moderation events with minimal evidence.
- Restrict first, investigate second for credible urgent reports.
- Never copy illegal-content samples into tickets, logs or federation payloads.
- Keep a contact for licence and privacy requests.

## 11. Troubleshooting

| Symptom | Check |
|---|---|
| `/readyz` fails | Database URL, migrations, storage permissions, disk space |
| Uploads 413 | `upload.max_file_size` and proxy body limit |
| Uploads 415 | Actual format not in `allowed_mimes` |
| Uploads 422 | Pixel/frame/memory limit; inspect the redacted detail |
| Media 404 after approval | Blob missing, status not active, or cache/proxy |
| Manifest signature invalid | Key file mismatch, clock skew, canonical JSON bug |
| Disk fills | Quota report, orphan detector, old quarantine, WAL |
| Federation silent | Signature verification, allow/block list, DNS, TLS |

## 12. Disaster recovery

- Database lost: restore the dump; blobs can be re-verified against hashes.
- Blob lost: mark the content unavailable; re-fetch from a replica if policy
  allows.
- Key lost: the node can no longer sign as its old identity; rotate and announce,
  but old signatures remain valid history.
- Compromise: block federation, rotate keys, preserve logs, notify peers, review
  content and access.

## 13. Legal and abuse

Publish a contact, a privacy notice, a takedown procedure and a content policy.
Keep records required by law, delete what is not required, and obtain legal
advice for the operator's jurisdiction. The software cannot make an operator
compliant by itself.

## 14. Open points

- Final command names and Compose service names.
- Whether `/metrics` is on a separate port by default.
- Backup encryption tooling and key custody.
- The minimum supported PostgreSQL version.
- Whether a node without a public IPv4 can federate through an outbound-only
  relay.
