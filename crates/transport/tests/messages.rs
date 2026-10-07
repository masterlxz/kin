use std::time::Duration;

use kin_identity::DeviceKey;
use kin_transport::{MAX_MESSAGE_SIZE, Node, NodeConfig, NodeEvent};
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(15);

fn node(key: &DeviceKey) -> Node {
    Node::new(key.keypair().clone(), NodeConfig { mdns: false }).unwrap()
}

/// Conecta `b` em `a` (loopback) e avança os dois até ambos verem a conexão.
async fn connected_pair() -> (DeviceKey, DeviceKey, Node, Node) {
    let (key_a, key_b) = (DeviceKey::generate(), DeviceKey::generate());
    let (mut a, mut b) = (node(&key_a), node(&key_b));
    a.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    let addr = loop {
        if let NodeEvent::Listening(addr) = a.next_event().await {
            break addr;
        }
    };
    b.dial(addr).unwrap();
    let (id_a, id_b) = (key_a.peer_id(), key_b.peer_id());
    let seen_a = async { while a.next_event().await != NodeEvent::PeerConnected(id_b) {} };
    let seen_b = async { while b.next_event().await != NodeEvent::PeerConnected(id_a) {} };
    timeout(WAIT, async { tokio::join!(seen_a, seen_b) })
        .await
        .expect("sem conexão");
    (key_a, key_b, a, b)
}

#[tokio::test(flavor = "multi_thread")]
async fn message_is_delivered_and_acked() {
    let (key_a, key_b, mut a, mut b) = connected_pair().await;
    let id = b.send(key_a.peer_id(), b"ola".to_vec());

    let received = async {
        loop {
            if let NodeEvent::MessageReceived { peer, data } = a.next_event().await {
                return (peer, data);
            }
        }
    };
    let acked = async {
        loop {
            if let NodeEvent::MessageDelivered { peer, id: got } = b.next_event().await {
                return (peer, got);
            }
        }
    };
    let ((from, data), (acker, got)) = timeout(WAIT, async { tokio::join!(received, acked) })
        .await
        .expect("sem entrega");
    assert_eq!(from, key_b.peer_id());
    assert_eq!(data, b"ola");
    assert_eq!(acker, key_a.peer_id());
    assert_eq!(got, id);
}

#[tokio::test(flavor = "multi_thread")]
async fn oversized_message_fails_to_send() {
    let (key_a, _key_b, mut a, mut b) = connected_pair().await;
    let id = b.send(key_a.peer_id(), vec![0; MAX_MESSAGE_SIZE + 1]);

    let failed = async {
        loop {
            if let NodeEvent::SendFailed { id: got, .. } = b.next_event().await {
                return got;
            }
        }
    };
    let drive_a = async {
        loop {
            a.next_event().await;
        }
    };
    let got = timeout(WAIT, async {
        tokio::select! { r = failed => r, _ = drive_a => unreachable!() }
    })
    .await
    .expect("sem falha");
    assert_eq!(got, id);
}
