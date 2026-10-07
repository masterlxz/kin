use std::time::Duration;

use kin_chat::{Chat, ChatEvent, Message, MessageId};
use kin_identity::{DeviceKey, IdentityId, IdentityProvider, StandaloneIdentity};
use kin_transport::{NodeConfig, PeerId};
use tokio::time::timeout;

const WAIT: Duration = Duration::from_secs(20);

struct Person {
    identity_id: IdentityId,
    peer_id: PeerId,
    chat: Chat,
}

fn person() -> Person {
    let identity = StandaloneIdentity::generate();
    let device = DeviceKey::generate();
    let chat = Chat::new(&identity, &device, NodeConfig { mdns: false }).unwrap();
    Person {
        identity_id: identity.id(),
        peer_id: device.peer_id(),
        chat,
    }
}

/// Avança o chat até um evento que o filtro aceita; descarta os demais.
async fn wait_for<T>(chat: &mut Chat, mut pick: impl FnMut(ChatEvent) -> Option<T>) -> T {
    loop {
        if let Some(found) = pick(chat.next_event().await) {
            return found;
        }
    }
}

/// Dois chats conectados em loopback, com a conversa E2EE pronta nos dois lados.
async fn ready_pair() -> (Person, Person) {
    let (mut a, mut b) = (person(), person());
    a.chat
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    let addr = wait_for(&mut a.chat, |e| match e {
        ChatEvent::Listening(addr) => Some(addr),
        _ => None,
    })
    .await;
    b.chat.dial(addr).unwrap();

    let (pa, pb) = (a.peer_id, b.peer_id);
    // Os dois nós precisam ser dirigidos juntos até ambos terminarem o handshake.
    let (mut ready_a, mut ready_b) = (None, None);
    timeout(WAIT, async {
        while ready_a.is_none() || ready_b.is_none() {
            tokio::select! {
                e = a.chat.next_event() => if let ChatEvent::ConversationReady { peer, identity } = e {
                    ready_a = Some((peer, identity));
                },
                e = b.chat.next_event() => if let ChatEvent::ConversationReady { peer, identity } = e {
                    ready_b = Some((peer, identity));
                },
            }
        }
    })
    .await
    .expect("handshake não terminou");
    assert_eq!(ready_a, Some((pb, b.identity_id.clone())));
    assert_eq!(ready_b, Some((pa, a.identity_id.clone())));
    (a, b)
}

async fn next_received(chat: &mut Chat) -> (IdentityId, Message) {
    wait_for(chat, |e| match e {
        ChatEvent::Received {
            sender, message, ..
        } => Some((sender, message)),
        _ => None,
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn handshake_then_messages_flow_both_ways() {
    let (mut a, mut b) = ready_pair().await;
    assert!(a.chat.is_ready(&b.peer_id) && b.chat.is_ready(&a.peer_id));

    let sent = a.chat.send(b.peer_id, "oi bia", None).unwrap();
    let delivered = wait_for(&mut a.chat, |e| match e {
        ChatEvent::Delivered { message, .. } => Some(message),
        _ => None,
    });
    let received = next_received(&mut b.chat);
    let (delivered, (sender, message)) = timeout(WAIT, async { tokio::join!(delivered, received) })
        .await
        .expect("sem entrega");
    assert_eq!(delivered, sent);
    assert_eq!(sender, a.identity_id);
    assert_eq!(message.id, sent);
    assert_eq!(message.text, "oi bia");
    assert_eq!(message.parent, None);

    // A resposta de volta usa o mesmo canal e referencia a mensagem original (thread).
    let reply = b.chat.send(a.peer_id, "oi ana", Some(sent)).unwrap();
    let (sender, message) = timeout(WAIT, async {
        let wait_a = next_received(&mut a.chat);
        let drive_b = async {
            loop {
                b.chat.next_event().await;
            }
        };
        tokio::select! { r = wait_a => r, _ = drive_b => unreachable!() }
    })
    .await
    .expect("sem resposta");
    assert_eq!(sender, b.identity_id);
    assert_eq!(message.id, reply);
    assert_eq!(message.parent, Some(sent));
}

#[tokio::test(flavor = "multi_thread")]
async fn send_without_conversation_fails() {
    let (mut a, b) = (person(), person());
    let err = a.chat.send(b.peer_id, "oi", None).unwrap_err();
    assert!(matches!(err, kin_chat::Error::NoConversation(p) if p == b.peer_id));
}

#[test]
fn message_format_roundtrip_and_rejects_bad_input() {
    assert!(Message::decode(&[]).is_err());
    assert!(Message::decode(&[2; 40]).is_err(), "versão desconhecida");

    let mut bytes = vec![1];
    bytes.extend([7u8; 16]);
    bytes.push(0);
    bytes.extend(42u64.to_be_bytes());
    bytes.extend(b"texto");
    let message = Message::decode(&bytes).unwrap();
    assert_eq!(message.encode(), bytes);
    assert_eq!(message.parent, None);
    assert_eq!(message.sent_at_ms, 42);

    let parent = MessageId::from_bytes([9; 16]);
    let mut with_parent = vec![1];
    with_parent.extend([7u8; 16]);
    with_parent.push(1);
    with_parent.extend(parent.as_bytes());
    with_parent.extend(42u64.to_be_bytes());
    with_parent.extend("ação".as_bytes());
    let message = Message::decode(&with_parent).unwrap();
    assert_eq!(message.parent, Some(parent));
    assert_eq!(message.text, "ação");
    assert_eq!(message.encode(), with_parent);

    assert!(Message::decode(&bytes[..20]).is_err(), "truncada");
    let mut bad_utf8 = bytes.clone();
    bad_utf8.push(0xff);
    assert!(Message::decode(&bad_utf8).is_err());
}
