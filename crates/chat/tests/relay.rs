//! Fase 2: a conversa E2EE (handshake MLS + verificação do Peer ID, P15) funciona por um relay.

mod common;

use common::{WAIT, connect_until_ready_within, receive_while_driving, wait_for};
use kin_chat::{Chat, ChatEvent};
use kin_identity::{DeviceKey, IdentityProvider, StandaloneIdentity};
use kin_transport::{Multiaddr, Node, NodeConfig, NodeEvent, Protocol, RelayLimits};
use tokio::time::timeout;

/// Sobe um nó relay em segundo plano e devolve o endereço dele (com `/p2p/<id>`).
async fn start_relay() -> Multiaddr {
    let key = DeviceKey::generate();
    let mut relay = Node::new(
        key.keypair().clone(),
        NodeConfig {
            mdns: false,
            relay_server: Some(RelayLimits::default()),
            ..Default::default()
        },
    )
    .unwrap();
    relay
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    let addr = loop {
        if let NodeEvent::Listening(addr) = relay.next_event().await {
            break addr;
        }
    };
    tokio::spawn(async move {
        loop {
            relay.next_event().await;
        }
    });
    addr.with(Protocol::P2p(key.peer_id()))
}

#[tokio::test(flavor = "multi_thread")]
async fn encrypted_conversation_works_through_a_relay() {
    let relay = start_relay().await;
    let (id_a, id_b) = (
        StandaloneIdentity::generate(),
        StandaloneIdentity::generate(),
    );
    let (dev_a, dev_b) = (DeviceKey::generate(), DeviceKey::generate());

    // A não escuta em endereço nenhum: só é alcançável pelo circuito do relay.
    let mut a = Chat::new(
        &id_a,
        &dev_a,
        NodeConfig {
            mdns: false,
            relays: vec![relay.clone()],
            ..Default::default()
        },
    )
    .unwrap();
    let mut b = Chat::new(
        &id_b,
        &dev_b,
        NodeConfig {
            mdns: false,
            ..Default::default()
        },
    )
    .unwrap();
    timeout(
        WAIT,
        wait_for(&mut a, |e| {
            matches!(e, ChatEvent::RelayReserved { .. }).then_some(())
        }),
    )
    .await
    .expect("sem reserva no relay");

    b.dial(a.circuit_address(&relay)).unwrap();
    let (seen_a, seen_b) = connect_until_ready_within(&mut a, &mut b, WAIT).await;
    let relayed = |events: &[ChatEvent]| {
        events
            .iter()
            .any(|e| matches!(e, ChatEvent::Route { relayed: true, .. }))
    };
    assert!(
        relayed(&seen_a) || relayed(&seen_b),
        "a conversa deveria ter passado pelo relay: {seen_a:?} / {seen_b:?}"
    );

    // O certificado (P15) vale igual por circuito: a identidade mostrada é a de quem está do outro lado.
    let sent = a.send(dev_b.peer_id(), "oi pelo relay", None).unwrap();
    let (sender, message) = receive_while_driving(&mut b, &mut a).await;
    assert_eq!((sender, message.id), (id_a.id(), sent));
    let sent = b.send(dev_a.peer_id(), "volta pelo relay", None).unwrap();
    let (sender, message) = receive_while_driving(&mut a, &mut b).await;
    assert_eq!((sender, message.id), (id_b.id(), sent));
}
