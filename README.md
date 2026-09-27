# Kin

A decentralized communication layer where the participating devices themselves are the
infrastructure. A messenger is the first application built on top of it; files, calls and
service discovery for the rest of the ecosystem come later, over the same network.

> **Status: pre-project.** The idea and spec are recorded; no code yet. The name is provisional.

## Why

Two people should be able to install an app and talk — end-to-end encrypted — without either of
them running a server, container or VPS. The network doesn't need to exist before its users: it
starts working with two peers and gets better (more paths, relays and redundancy) as it grows.

## Principles

- **Decentralized ≠ no infrastructure.** Bootstrap nodes and dedicated relays may exist; none is
  indispensable.
- **Graceful degradation.** Local (LAN) → direct → relay by a peer → dedicated relay.
- **Bounded contribution.** Every open app can relay, store and route for others — within limits.
- **E2EE everywhere.** Relays only see encrypted bytes.
- **Identity is a key, not an account** — via [TruthID](https://github.com/masterlxz/truthid).
- **No blockchain in the message path.** Ever.

## Roadmap

| Phase | Delivers |
|---|---|
| 1 | Local identity + direct connection (LAN/internet) + 1:1 E2EE chat |
| 2 | Hole punching + relays + QR/link invites |
| 3 | Store-and-forward (offline delivery) + multi-device |
| 4 | Overlay: gossip + DHT + app-as-relay with limits |
| 5 | Groups/channels, files, presence, notifications |
| 6 | WebRTC calls + opt-in TURN; packaged dedicated relay |
| 7–8 | *(optional)* Private multi-hop routing; on-chain registry/incentives |

Full planning (in Portuguese) lives in [`project/`](project/INDEX.md).

## Ecosystem

Part of an open-source decentralized ecosystem: [TruthID](https://github.com/masterlxz/truthid)
(identity), [Warden](https://github.com/masterlxz/warden) (personal AI agent),
[Anchor](https://github.com/masterlxz/anchor) and Lume.

## License

[MIT](LICENSE)
