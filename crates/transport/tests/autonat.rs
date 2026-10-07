//! Fase 2: o AutoNAT conclui que um nó com porta alcançável é público.

use std::time::Duration;

use kin_identity::DeviceKey;
use kin_transport::{NatStatus, Node, NodeConfig, NodeEvent};
use tokio::time::timeout;

fn node(key: &DeviceKey) -> Node {
    Node::new(
        key.keypair().clone(),
        NodeConfig {
            mdns: false,
            // Tudo aqui é loopback; sem isto o AutoNAT ignora os endereços (não são globais).
            autonat_global_only: false,
            ..Default::default()
        },
    )
    .unwrap()
}

/// O AutoNAT espera 15 s antes da primeira sondagem (`boot_delay` do libp2p), por isso o prazo longo.
#[tokio::test(flavor = "multi_thread")]
async fn node_with_a_reachable_port_is_reported_public() {
    let (key_client, key_server) = (DeviceKey::generate(), DeviceKey::generate());
    let (mut client, mut server) = (node(&key_client), node(&key_server));
    for n in [&mut client, &mut server] {
        n.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
            .unwrap();
    }
    let server_addr = loop {
        if let NodeEvent::Listening(addr) = server.next_event().await {
            break addr;
        }
    };
    client.dial(server_addr).unwrap();

    let status = timeout(Duration::from_secs(60), async {
        loop {
            tokio::select! {
                e = server.next_event() => { let _ = e; }
                e = client.next_event() => match e {
                    NodeEvent::NatStatus(s) if s != NatStatus::Unknown => return s,
                    _ => {}
                },
            }
        }
    })
    .await
    .expect("o AutoNAT não chegou a uma conclusão");
    assert_eq!(status, NatStatus::Public);
}
