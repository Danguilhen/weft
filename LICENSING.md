# Licensing

Weft deliberately uses two software licences.

| Part | Licence | Why |
|---|---|---|
| `weft-server`, `weft-db`, `weft-storage`, `weft-validation`, `weft-federation`, `weft-cli` | AGPL-3.0-or-later | A hosted network service should share modifications with its users. |
| `weft-core`, `weft-client` | MIT | Clients such as Jabu must be able to embed the client and domain types without copyleft obligations. |

The root [`LICENSE`](LICENSE) is the AGPL-3.0 text. [`LICENSE-MIT`](LICENSE-MIT)
is the MIT text used by the client-side crates.

Every source file must carry an SPDX header:

```rust
// SPDX-License-Identifier: AGPL-3.0-or-later
```

or, for the MIT crates:

```rust
// SPDX-License-Identifier: MIT
```

Do not copy AGPL server code into `weft-core` or `weft-client`. If shared code is
needed by both sides, it belongs in `weft-core` under MIT.

## Contributions

By contributing, you agree that your contribution is licensed under the licence
of the crate it touches. There is no copyright assignment and no contributor
licence agreement. Keep the SPDX headers intact.

## Content

The software licence does not determine the licence of media uploaded to a
node. Uploaded media carries a **claimed** content licence, evidence and a
verification status. See [`docs/CONTENT_LICENSING.md`](docs/CONTENT_LICENSING.md).

## Documentation

The documentation is proposed as **CC BY-SA 4.0**. This is recorded in
[`docs/OPEN_QUESTIONS.md`](docs/OPEN_QUESTIONS.md) until confirmed before the
first public release.

## Name and marks

The Weft name, logo and domain policy are not settled. Do not register a domain,
crate name or trademark on behalf of the project without maintainer agreement.
