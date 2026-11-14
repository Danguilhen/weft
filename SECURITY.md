# Security policy

Weft is in the design phase. There is no deployed reference service yet, but the
protocol and the future server are security-sensitive by nature: they accept
untrusted media, federate with untrusted nodes and cache content.

## Reporting a vulnerability

Do **not** open a public issue for a vulnerability.

Once the repository has a public home, use a **private security advisory** on
that repository. Until then, contact the maintainers through the private channel
listed in the repository profile. Include:

- affected component and version or commit;
- a reproduction or proof of concept;
- impact and required preconditions;
- any suggested mitigation;
- whether you intend to disclose publicly and on what timeline.

Do not include real personal data, illegal-content samples or third-party
secrets in a report. Use synthetic test media.

## Scope

In scope once code exists:

- upload validation and decoder sandboxing;
- canonical JSON and Ed25519 signature verification;
- ActivityPub inbox authentication, replay protection and SSRF;
- media manifest and receipt forgery;
- replication quota and storage-exhaustion bypasses;
- rate-limit bypasses and anonymous-upload abuse;
- privacy leaks (IP addresses, JIDs, contributor identifiers);
- admin authentication and authorization;
- path traversal and unsafe file handling.

Out of scope for the project itself:

- the security of independently operated nodes;
- content licensing disputes (use the content report flow);
- denial-of-service attacks that require unbounded resources without a
  practical mitigation;
- vulnerabilities in third-party dependencies that are already public and
  fixed upstream (report them upstream, and tell us if Weft needs to update).

## Response

Maintainers aim to acknowledge a report within 7 days and to agree a disclosure
timeline with the reporter. Timelines may be longer for protocol-level issues
that require a versioned change across nodes.

We support the latest released minor version and the `main` branch. During the
design phase, the design documents are the supported artifact.

## Safe harbour

We will not pursue legal action against good-faith research that follows this
policy, avoids privacy violations and does not disrupt other users or nodes. Do
not test against nodes you do not operate without permission.
