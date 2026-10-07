use kin_crypto::{Conversation, CryptoDevice, Decrypted, Error};
use kin_identity::{IdentityProvider, StandaloneIdentity};

struct Person {
    identity: StandaloneIdentity,
    device: CryptoDevice,
}

fn person() -> Person {
    let identity = StandaloneIdentity::generate();
    let device = CryptoDevice::new(&identity, 1_000).unwrap();
    Person { identity, device }
}

/// Ana convida Bia e as duas passam a ter a mesma conversa.
fn pair() -> (Person, Person, Conversation, Conversation) {
    let (ana, bia) = (person(), person());
    let mut conv_ana = Conversation::create(&ana.device).unwrap();
    let invite = conv_ana
        .invite(&bia.device.key_package().unwrap(), Some(&bia.identity.id()))
        .unwrap();
    let conv_bia =
        Conversation::join(&bia.device, &invite.welcome, Some(&ana.identity.id())).unwrap();
    (ana, bia, conv_ana, conv_bia)
}

fn text(d: Decrypted) -> Vec<u8> {
    match d {
        Decrypted::Message { plaintext, .. } => plaintext,
        other => panic!("esperava mensagem, veio {other:?}"),
    }
}

#[test]
fn messages_flow_both_ways_and_carry_the_sender_identity() {
    let (ana, bia, mut c_ana, mut c_bia) = pair();
    assert_eq!(c_ana.id(), c_bia.id());
    assert_eq!(c_ana.epoch(), c_bia.epoch());
    assert_eq!(c_ana.peers().unwrap(), vec![bia.identity.id()]);
    assert_eq!(c_bia.peers().unwrap(), vec![ana.identity.id()]);

    let wire = c_ana.encrypt(b"oi bia").unwrap();
    assert!(
        !wire.windows(6).any(|w| w == b"oi bia"),
        "texto claro vazou"
    );
    match c_bia.decrypt(&wire).unwrap() {
        Decrypted::Message { sender, plaintext } => {
            assert_eq!(sender, ana.identity.id());
            assert_eq!(plaintext, b"oi bia");
        }
        other => panic!("{other:?}"),
    }

    let reply = c_bia.encrypt(b"oi ana").unwrap();
    assert_eq!(text(c_ana.decrypt(&reply).unwrap()), b"oi ana");
}

#[test]
fn tampered_ciphertext_is_rejected() {
    let (_, _, mut c_ana, mut c_bia) = pair();
    let mut wire = c_ana.encrypt(b"segredo").unwrap();
    let last = wire.len() - 1;
    wire[last] ^= 0xff;
    assert!(c_bia.decrypt(&wire).is_err());
    assert!(matches!(c_bia.decrypt(b"lixo"), Err(Error::Malformed)));
}

#[test]
fn redelivery_is_reported_as_duplicate_and_does_not_break_the_conversation() {
    let (_, _, mut c_ana, mut c_bia) = pair();
    let wire = c_ana.encrypt(b"uma vez so").unwrap();
    assert_eq!(text(c_bia.decrypt(&wire).unwrap()), b"uma vez so");
    assert!(matches!(c_bia.decrypt(&wire), Err(Error::Duplicate)));

    let next = c_ana.encrypt(b"seguinte").unwrap();
    assert_eq!(text(c_bia.decrypt(&next).unwrap()), b"seguinte");
}

#[test]
fn out_of_order_delivery_within_tolerance_still_decrypts() {
    let (_, _, mut c_ana, mut c_bia) = pair();
    let m1 = c_ana.encrypt(b"1").unwrap();
    let m2 = c_ana.encrypt(b"2").unwrap();
    let m3 = c_ana.encrypt(b"3").unwrap();
    assert_eq!(text(c_bia.decrypt(&m3).unwrap()), b"3");
    assert_eq!(text(c_bia.decrypt(&m1).unwrap()), b"1");
    assert_eq!(text(c_bia.decrypt(&m2).unwrap()), b"2");
}

#[test]
fn message_from_another_conversation_is_rejected() {
    let (_, _, _, mut c_bia) = pair();
    let (_, _, mut other_ana, _) = pair();
    let wire = other_ana.encrypt(b"nao e pra voce").unwrap();
    assert!(matches!(
        c_bia.decrypt(&wire),
        Err(Error::WrongConversation)
    ));
}

#[test]
fn key_rotation_advances_the_epoch_on_both_sides() {
    let (_, _, mut c_ana, mut c_bia) = pair();
    let before = c_ana.epoch();

    let commit = c_ana.rotate_keys().unwrap();
    assert_eq!(c_ana.epoch(), before + 1);
    assert_eq!(c_bia.decrypt(&commit).unwrap(), Decrypted::EpochChanged);
    assert_eq!(c_bia.epoch(), c_ana.epoch());

    let wire = c_bia.encrypt(b"depois da rotacao").unwrap();
    assert_eq!(text(c_ana.decrypt(&wire).unwrap()), b"depois da rotacao");
}

#[test]
fn late_message_from_the_previous_epoch_still_decrypts() {
    let (_, _, mut c_ana, mut c_bia) = pair();
    let late = c_ana.encrypt(b"enviada antes do commit").unwrap();
    let commit = c_ana.rotate_keys().unwrap();

    // O commit chega antes da mensagem (sem ordem garantida).
    c_bia.decrypt(&commit).unwrap();
    assert_eq!(
        text(c_bia.decrypt(&late).unwrap()),
        b"enviada antes do commit"
    );
}

#[test]
fn message_from_a_future_epoch_is_reported_until_the_commit_arrives() {
    let (_, _, mut c_ana, mut c_bia) = pair();
    let commit = c_ana.rotate_keys().unwrap();
    let early = c_ana.encrypt(b"nova epoca").unwrap();

    assert!(matches!(c_bia.decrypt(&early), Err(Error::UnknownEpoch)));
    c_bia.decrypt(&commit).unwrap();
    assert_eq!(text(c_bia.decrypt(&early).unwrap()), b"nova epoca");
}

#[test]
fn welcome_is_useless_to_a_device_it_was_not_made_for() {
    let (ana, bia) = (person(), person());
    let intruder = person();
    let mut conv = Conversation::create(&ana.device).unwrap();
    let invite = conv
        .invite(&bia.device.key_package().unwrap(), None)
        .unwrap();

    assert!(matches!(
        Conversation::join(&intruder.device, &invite.welcome, None),
        Err(Error::NotForThisDevice)
    ));
}

#[test]
fn identity_expectations_are_enforced_on_invite_and_join() {
    let (ana, bia) = (person(), person());
    let stranger = person();
    let mut conv = Conversation::create(&ana.device).unwrap();

    let kp = bia.device.key_package().unwrap();
    assert!(matches!(
        conv.invite(&kp, Some(&stranger.identity.id())),
        Err(Error::UnexpectedIdentity)
    ));
    assert!(matches!(conv.invite(b"lixo", None), Err(Error::Malformed)));

    let invite = conv.invite(&kp, Some(&bia.identity.id())).unwrap();
    assert!(matches!(
        Conversation::join(&bia.device, &invite.welcome, Some(&stranger.identity.id())),
        Err(Error::UnexpectedIdentity)
    ));
}
