# Weft — privacy model

Status: design. This is a privacy-preserving network by intent, not by
assumption. Decentralisation does not make privacy automatic.

## 1. Data Weft collects

| Data | Purpose | Retention | Visibility |
|---|---|---|---|
| Media bytes | Hosting and serving | Until removed or evicted | Public when active |
| Title, description, tags | Discovery | Same as content | Public |
| Licence claim and evidence | Transparency | Same as content | Public |
| Moderation events | Accountability | Permanent or long-lived | Public when federated |
| Report text and optional contact | Abuse handling | Configured (default 90 days) | Operator only |
| Salted client hash | Rate limiting | Configured (default 7 days) | Operator only |
| Access logs | Operations and abuse | Short (default 7 days) | Operator only |
| Node descriptor and keys | Federation | Public | Public |
| Transfer receipts | Local accounting | Experimental, bounded | Peers and operator |
| Admin audit log | Accountability | Long-lived | Operator only |

We do **not** collect:

- accounts, passwords or email addresses;
- XMPP JIDs or ActivityPub actor identities from viewers;
- advertising identifiers, analytics or cross-site cookies;
- contact lists, device fingerprints or precise location;
- third-party provider tokens.

## 2. Data Weft does not federate

Never propagate through ActivityPub:

- XMPP JIDs, email addresses, real names or phone numbers;
- raw IP addresses or user agents;
- private report contents or reporter contact details;
- admin identities or tokens;
- a contributor identity unless the contributor explicitly chose a pseudonymous
  public key and the node policy allows it.

Node identity is public by design. Contributor identity is optional and separate.

## 3. Anonymous contributions

- No account is required to browse, search, download or (policy permitting)
  upload.
- Anonymous uploads are rate-limited by a salted, rotating hash of the client
  address. The raw address is not stored.
- The salt is stored root-only and rotated; old hashes stop correlating.
- An optional proof-of-work challenge can replace or supplement IP-based limits.
- Optional pseudonymous contributor keys establish a contribution history without
  a real identity. They are separate from node keys, opt-in and not federated by
  default.
- The node may keep abuse evidence privately where legally required, for the
  shortest period allowed.

## 4. Anonymity limits

Document these honestly to users and operators:

- **Network observers** can correlate traffic timing, sizes and paths.
- **The node operator** sees the client address of every direct request unless a
  proxy hides it.
- **A reverse proxy or CDN** sees the same traffic.
- **Federated public metadata** (manifests, events) is public forever.
- **Receipts** link two nodes, not users, but can reveal traffic patterns.
- **A downloaded file** can be re-shared elsewhere; Weft cannot recall it.
- **Illegal-content investigations** may require limited data retention; do not
  promise absolute anonymity.

## 5. Legal bases and rights

The operator is the data controller for its node. Before public deployment, the
operator must:

- document a lawful basis for abuse-related processing;
- provide a privacy notice and a contact;
- honour applicable access, erasure and objection rights without breaking the
  public nature of federated metadata;
- define a retention schedule and delete on time;
- avoid transferring personal data to third parties;
- obtain legal advice for its jurisdiction (for example UK GDPR and the Online
  Safety Act).

The software must make data minimisation easy: short retention defaults, no raw
IPs, no analytics and no mandatory accounts.

## 6. Operator checklist

- [ ] TLS is enabled and HSTS is set at the proxy.
- [ ] Access logs are rotated and do not include query strings with sensitive
      data.
- [ ] The hash salt is root-only and rotated on schedule.
- [ ] Client-hash and report retention match the privacy notice.
- [ ] Backups are encrypted and access-controlled.
- [ ] No third-party embeds or analytics are added to the API or admin surface.
- [ ] Federation metadata contains no personal data.
- [ ] A takedown and illegal-content procedure exists.

## 7. Open questions

- Default retention for salted client hashes and reports.
- Whether anonymous uploads require proof of work.
- Whether receipts should be public or operator-only.
- Whether optional contributor keys should be supported in the MVP (currently
  no).
- The privacy notice template and jurisdiction-specific wording.
