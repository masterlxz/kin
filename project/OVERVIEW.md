# O que é o Kin

Uma **camada de comunicação descentralizada** em que os próprios dispositivos participantes formam a
infraestrutura. O mensageiro é a **primeira aplicação** construída sobre ela; arquivos, chamadas e
descoberta de serviços do ecossistema vêm depois, sobre a mesma rede.

Para o usuário, é um app de chat comum ("Fabio 🟢 online — Oi!"). DHT, relays e roteamento ficam
invisíveis.

**Propriedade central — *zero-infrastructure bootstrap***: a rede não precisa existir antes dos
usuários. Ela nasce quando eles começam a participar, melhora com o crescimento, mas **funciona com 2**.

Stack planejada (ainda não decidida — ver D3 em `ARCHITECTURE.md`):
- **Núcleo de rede**: Rust (`rust-libp2p`; alternativa a avaliar: `iroh`)
- **Desktop**: Tauri (mesmo padrão do TruthID/Warden/Anchor)
- **Mobile**: Flutter via FFI/`flutter_rust_bridge` (mesmo padrão do TruthID)
- **E2EE**: MLS (RFC 9420, ex. `openmls`) vs Double Ratchet — ver D2
- **Chamadas**: WebRTC + STUN/TURN (Fase 6)
- **Identidade**: TruthID (modo de identidade local, sem on-chain no MVP)
- **Mailbox offline**: em aberto — hipótese "outbox local + relays Nostr como fallback" (D1)

**Ecossistema**: TruthID (identidade), Warden, Anchor, Lume. O Kin é um projeto **separado** do
TruthID e o consome — não é um módulo de comunicação dentro do TruthID (ideia já descartada antes).

---

# Status Geral

```
Fase 1 — Identidade local + conexão direta + chat 1:1 E2EE    [ ] Pendente
Fase 2 — Hole punching + relay + convite por QR/link          [ ] Pendente
Fase 3 — Store-and-forward (mailbox) + multi-device           [ ] Pendente
Fase 4 — Overlay: gossip + DHT + app-relay com limites        [ ] Pendente
Fase 5 — Grupos/canais, arquivos, presença, notificações      [ ] Pendente
Fase 6 — Chamadas WebRTC + TURN opt-in; relay empacotado      [ ] Pendente
Fase 7 — (opcional) Roteamento multi-hop / privado            [ ] Pendente
Fase 8 — (opcional) Blockchain: registro, proof-of-relay      [ ] Pendente
```

Próximo passo: estudar o prior art (`ROADMAP.md`) e fechar as decisões D1–D4 (`ARCHITECTURE.md`)
antes de iniciar a Fase 1.
