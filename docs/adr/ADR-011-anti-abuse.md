# ADR-011 — Anonymous-upload anti-abuse: token buckets, quotas, optional PoW

- Status: Accepted
- Date: 2026-11-14
- Affects: API, privacy, operations, MVP

## Context

Anonymous uploads need abuse controls that do not require accounts, a central
CAPTCHA service or raw IP retention. IP-only limits are weak against distributed
abuse and unfair behind NAT; proof of work costs CPU and battery on small devices.
Review mode (ADR-010) is the primary control, but a public node still needs
bounded resource use.

## Decision

- Use a **salted, rotating hash** of the client address for rate limiting. No raw
  IP address is stored.
- Default bucket: **20 uploads per hour, burst 5**. IPv4 is per address; IPv6 is
  per `/64` prefix by default (configurable), and the trade-off is documented.
- Per-client quotas: **10 pending uploads** and **100 MB per day** (configurable).
- A global quarantine cap and expiry (default 7 days) bound storage use.
- **Proof of work is optional and off by default.** A node may require it for
  `open` uploads; it must not be mandatory for `review` on a small node.
- No third-party CAPTCHA or analytics.
- The hash salt is root-only, rotated weekly, and old buckets expire with the
  documented retention.

## Consequences

- Positive: cheap, privacy-preserving baseline that works on a small node.
- Positive: review mode plus quotas limits the damage of a distributed flood.
- Negative: NAT and IPv6 privacy addresses can still cause false positives.
- Negative: a determined botnet can distribute uploads; quotas and review remain
  the real defence.
- Follow-up: measure false positives on a shared network and tune defaults.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Raw IP logging | Privacy violation and unnecessary retention |
| Mandatory PoW | Battery/CPU cost; accessibility; botnets can still pay |
| Central CAPTCHA | Third-party dependency and privacy loss |
| Accounts | Violates anonymous contribution |
| No limits | Storage/queue exhaustion |

## References

- [`ADR-005`](ADR-005-anonymous-uploads.md)
- [`ADR-010`](ADR-010-upload-policy.md)
- [`../PRIVACY.md`](../PRIVACY.md)
- [`../CONFIG.md`](../CONFIG.md)
