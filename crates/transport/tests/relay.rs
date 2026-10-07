//! Fase 2: três nós em loopback (relay R, A e B). A e B só se alcançam passando por R.

use std::time::Duration;

use kin_identity::DeviceKey;
use kin_transport::{Multiaddr, Node, NodeConfig, NodeEvent, PeerId, Protocol, RelayLimits};
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(30);

fn node(key: &DeviceKey, config: NodeConfig) -> Node {
    Node::new(key.keypair().clone(), config).unwrap()
}

fn plain() -> NodeConfig {
    NodeConfig {
        mdns: false,
        ..Default::default()
    }
}

/// Dirige o nó até um evento que o filtro aceita.
async fn until<T>(node: &mut Node, mut pick: impl FnMut(NodeEvent) -> Option<T>) -> T {
    loop {
        if let Some(found) = pick(node.next_event().await) {
            return found;
        }
    }
}

async fn listen_loopback(node: &mut Node) -> Multiaddr {
    node.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    until(node, |e| match e {
        NodeEvent::Listening(addr) => Some(addr),
        _ => None,
    })
    .await
}

/// Sobe o relay em segundo plano e devolve o endereço dele (já com `/p2p/<id>`).
async fn start_relay(limits: RelayLimits) -> Multiaddr {
    let key = DeviceKey::generate();
    let mut relay = node(
        &key,
        NodeConfig {
            relay_server: Some(limits),
            ..plain()
        },
    );
    let addr = listen_loopback(&mut relay)
        .await
        .with(Protocol::P2p(key.peer_id()));
    tokio::spawn(async move {
        loop {
            relay.next_event().await;
        }
    });
    addr
}

/// `a` pede reserva no relay; devolve quando o relay aceita.
async fn reserve(a: &mut Node) {
    timeout(
        WAIT,
        until(a, |e| {
            matches!(e, NodeEvent::RelayReserved { .. }).then_some(())
        }),
    )
    .await
    .expect("o relay não aceitou a reserva");
}

/// Dirige os dois nós até ambos verem a conexão um com o outro; `true` se for via relay.
async fn connect_via_relay(a: &mut Node, b: &mut Node, a_id: PeerId, b_id: PeerId) -> bool {
    let (mut a_up, mut b_up, mut relayed) = (false, false, false);
    timeout(WAIT, async {
        while !(a_up && b_up) {
            tokio::select! {
                e = a.next_event() => match e {
                    NodeEvent::PeerConnected(p) if p == b_id => a_up = true,
                    NodeEvent::PeerRoute { relayed: r, .. } => relayed |= r,
                    _ => {}
                },
                e = b.next_event() => match e {
                    NodeEvent::PeerConnected(p) if p == a_id => b_up = true,
                    NodeEvent::PeerRoute { relayed: r, .. } => relayed |= r,
                    _ => {}
                },
            }
        }
    })
    .await
    .expect("A e B não se conectaram pelo relay");
    relayed
}

/// A (sem endereço direto, só reserva no relay) e B (que só conhece o relay).
struct Trio {
    a: Node,
    b: Node,
    a_id: PeerId,
    b_id: PeerId,
}

async fn trio(limits: RelayLimits) -> Trio {
    let relay = start_relay(limits).await;
    let (key_a, key_b) = (DeviceKey::generate(), DeviceKey::generate());
    let mut a = node(
        &key_a,
        NodeConfig {
            relays: vec![relay.clone()],
            ..plain()
        },
    );
    let mut b = node(&key_b, plain());
    reserve(&mut a).await;
    b.dial(a.circuit_address(&relay)).unwrap();
    let relayed = connect_via_relay(&mut a, &mut b, key_a.peer_id(), key_b.peer_id()).await;
    assert!(relayed, "a conexão deveria passar pelo relay");
    Trio {
        a,
        b,
        a_id: key_a.peer_id(),
        b_id: key_b.peer_id(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn message_travels_through_the_relay_and_is_acked() {
    let Trio {
        mut a,
        mut b,
        a_id,
        b_id,
    } = trio(RelayLimits::default()).await;

    let id = b.send(a_id, b"oi pelo relay".to_vec());
    let (delivered, received) = timeout(WAIT, async {
        let got = async {
            until(&mut a, |e| match e {
                NodeEvent::MessageReceived { peer, data } => Some((peer, data)),
                _ => None,
            })
            .await
        };
        let acked = async {
            until(&mut b, |e| match e {
                NodeEvent::MessageDelivered { id: got, .. } => Some(got),
                _ => None,
            })
            .await
        };
        tokio::join!(acked, got)
    })
    .await
    .expect("sem entrega pelo relay");
    assert_eq!(delivered, id);
    assert_eq!(received, (b_id, b"oi pelo relay".to_vec()));
}

#[tokio::test(flavor = "multi_thread")]
async fn relay_without_byte_limit_carries_a_large_message() {
    let Trio {
        mut a, mut b, a_id, ..
    } = trio(RelayLimits::default()).await;

    // Bem acima dos 128 KiB do padrão do libp2p, em várias mensagens de 60 KiB.
    for _ in 0..5 {
        b.send(a_id, vec![7u8; 60 * 1024]);
    }
    let mut delivered = 0;
    timeout(WAIT, async {
        while delivered < 5 {
            tokio::select! {
                e = a.next_event() => { let _ = e; }
                e = b.next_event() => match e {
                    NodeEvent::MessageDelivered { .. } => delivered += 1,
                    NodeEvent::SendFailed { reason, .. } => panic!("falhou: {reason}"),
                    _ => {}
                },
            }
        }
    })
    .await
    .expect("o relay não carregou as mensagens");
}

#[tokio::test(flavor = "multi_thread")]
async fn relay_with_a_byte_limit_cuts_the_circuit() {
    let Trio {
        mut a, mut b, a_id, ..
    } = trio(RelayLimits {
        max_circuit_bytes: 2 * 1024,
        ..Default::default()
    })
    .await;

    b.send(a_id, vec![7u8; 30 * 1024]);
    let outcome = timeout(WAIT, async {
        loop {
            tokio::select! {
                e = a.next_event() => { let _ = e; }
                e = b.next_event() => match e {
                    NodeEvent::MessageDelivered { .. } => return "delivered",
                    NodeEvent::SendFailed { .. } => return "failed",
                    _ => {}
                },
            }
        }
    })
    .await
    .expect("nem entregou nem falhou");
    assert_eq!(outcome, "failed", "o limite de bytes do relay não valeu");
}

/// Com endereços diretos nos dois lados (aqui, loopback), o DCUtR troca o relay por uma conexão
/// direta sem ninguém pedir.
#[tokio::test(flavor = "multi_thread")]
async fn hole_punching_replaces_the_relay_with_a_direct_connection() {
    let relay = start_relay(RelayLimits::default()).await;
    let (key_a, key_b) = (DeviceKey::generate(), DeviceKey::generate());
    let mut a = node(
        &key_a,
        NodeConfig {
            relays: vec![relay.clone()],
            ..plain()
        },
    );
    let mut b = node(&key_b, plain());
    // Os dois escutam em loopback: são os endereços que o DCUtR vai tentar.
    listen_loopback(&mut a).await;
    listen_loopback(&mut b).await;
    reserve(&mut a).await;
    b.dial(a.circuit_address(&relay)).unwrap();

    let (mut punched, mut direct) = (false, false);
    timeout(WAIT, async {
        while !(punched && direct) {
            tokio::select! {
                e = a.next_event() => match e {
                    NodeEvent::HolePunch { result, .. } => punched |= result.is_ok(),
                    NodeEvent::PeerRoute { relayed: false, peer } if peer == key_b.peer_id() => direct = true,
                    _ => {}
                },
                e = b.next_event() => if let NodeEvent::HolePunch { result, .. } = e { punched |= result.is_ok() },
            }
        }
    })
    .await
    .expect("o hole punching não trocou o relay por uma conexão direta");
}

/// Discar o circuito antes de a reserva sair não pode ser cancelado: o `Node` espera sozinho.
#[tokio::test(flavor = "multi_thread")]
async fn circuit_dial_issued_before_the_reservation_waits_for_it() {
    let relay = start_relay(RelayLimits::default()).await;
    let (key_a, key_b) = (DeviceKey::generate(), DeviceKey::generate());
    let with_relay = |r: &Multiaddr| NodeConfig {
        relays: vec![r.clone()],
        ..plain()
    };
    let mut a = node(&key_a, with_relay(&relay));
    reserve(&mut a).await;

    // B também usa o relay e disca A NA HORA, com a própria reserva ainda a caminho.
    let mut b = node(&key_b, with_relay(&relay));
    b.dial(a.circuit_address(&relay)).unwrap();
    let relayed = connect_via_relay(&mut a, &mut b, key_a.peer_id(), key_b.peer_id()).await;
    assert!(relayed);
}

#[tokio::test(flavor = "multi_thread")]
async fn shareable_addresses_lists_external_and_circuit_but_not_loopback() {
    let relay = start_relay(RelayLimits::default()).await;
    let key = DeviceKey::generate();
    let public: Multiaddr = "/ip4/203.0.113.7/tcp/4001".parse().unwrap();
    let mut a = node(
        &key,
        NodeConfig {
            relays: vec![relay],
            external_addrs: vec![public.clone()],
            ..plain()
        },
    );
    listen_loopback(&mut a).await;
    reserve(&mut a).await;

    let shared = a.shareable_addresses();
    assert!(
        shared.contains(&public.with(Protocol::P2p(key.peer_id()))),
        "{shared:?}"
    );
    // O relay do teste está em loopback, então o circuito dele e o endereço de escuta não entram.
    assert!(
        shared
            .iter()
            .all(|addr| !addr.to_string().contains("127.0.0.1")),
        "{shared:?}"
    );
}
