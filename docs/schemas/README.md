# Wire schemas

Draft machine-readable schemas for the Weft protocol. They are design artifacts
and must be updated with the implementation.

| File | Describes |
|---|---|
| [`manifest.schema.json`](manifest.schema.json) | Signed media manifest (`weft-manifest/0.1`) |
| [`node.schema.json`](node.schema.json) | Node descriptor served at `/.well-known/weft` |

Both schemas use JSON Schema 2020-12. They allow unknown properties so a minor
protocol version can add fields; consumers must ignore what they do not know.

## Validation

There is no schema validation in CI yet. When the Rust workspace exists, the
golden-vector tests should validate:

- a signed manifest round trip;
- a node descriptor with a rotated key;
- rejection of a manifest with a mismatched hash;
- rejection of an unknown signature algorithm.

Do not edit a schema without updating [`../PROTOCOL.md`](../PROTOCOL.md) and
[`../API.md`](../API.md).
