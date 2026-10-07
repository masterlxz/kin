use kin_identity::{DeviceKey, IdentityProvider, StandaloneIdentity};

#[test]
fn identity_signature_verifies_and_rejects_tampering() {
    let id = StandaloneIdentity::generate();
    let sig = id.sign(b"ola").unwrap();
    assert!(id.id().public_key().verify(b"ola", &sig));
    assert!(!id.id().public_key().verify(b"outra", &sig));
}

#[test]
fn device_certificate_verifies() {
    let id = StandaloneIdentity::generate();
    let device = DeviceKey::generate();
    let cert = id.authorize_device(&device.public_key(), 1_000).unwrap();
    cert.verify().unwrap();
    assert_eq!(cert.identity, id.id());
    assert_eq!(cert.device_peer_id(), device.peer_id());
}

#[test]
fn certificate_from_another_identity_or_tampered_fails() {
    let id = StandaloneIdentity::generate();
    let other = StandaloneIdentity::generate();
    let device = DeviceKey::generate();

    let mut cert = id.authorize_device(&device.public_key(), 1_000).unwrap();
    cert.created_at += 1;
    assert!(cert.verify().is_err());

    let mut forged = id.authorize_device(&device.public_key(), 1_000).unwrap();
    forged.identity = other.id();
    assert!(forged.verify().is_err());

    let mut swapped = id.authorize_device(&device.public_key(), 1_000).unwrap();
    swapped.device = DeviceKey::generate().public_key();
    assert!(swapped.verify().is_err());
}

#[test]
fn peer_id_is_stable_across_serialization() {
    let device = DeviceKey::generate();
    let restored = DeviceKey::from_bytes(&device.to_bytes().unwrap()).unwrap();
    assert_eq!(device.peer_id(), restored.peer_id());
}

#[test]
fn identity_roundtrips_through_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.key");
    let id = StandaloneIdentity::generate();
    id.save(&path).unwrap();

    let loaded = StandaloneIdentity::load(&path).unwrap();
    assert_eq!(loaded.id(), id.id());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}

#[test]
fn rejects_non_ed25519_or_garbage_bytes() {
    assert!(StandaloneIdentity::from_bytes(b"lixo").is_err());
    assert!(DeviceKey::from_bytes(&[]).is_err());
}

#[test]
fn certificate_roundtrips_through_bytes_and_still_verifies() {
    let id = StandaloneIdentity::generate();
    let device = DeviceKey::generate();
    let cert = id.authorize_device(&device.public_key(), 42).unwrap();

    let restored = kin_identity::DeviceCertificate::from_bytes(&cert.to_bytes()).unwrap();
    assert_eq!(restored, cert);
    restored.verify().unwrap();
}

#[test]
fn certificate_from_bytes_rejects_truncated_and_trailing_data() {
    let id = StandaloneIdentity::generate();
    let cert = id
        .authorize_device(&DeviceKey::generate().public_key(), 1)
        .unwrap();
    let bytes = cert.to_bytes();

    assert!(kin_identity::DeviceCertificate::from_bytes(&bytes[..bytes.len() - 1]).is_err());
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(kin_identity::DeviceCertificate::from_bytes(&extra).is_err());
    assert!(kin_identity::DeviceCertificate::from_bytes(&[]).is_err());
}

#[test]
fn ed25519_key_helpers_roundtrip() {
    let key = DeviceKey::generate().public_key();
    let raw = kin_identity::ed25519_bytes(&key).unwrap();
    assert_eq!(kin_identity::public_key_from_ed25519(&raw).unwrap(), key);
    assert!(kin_identity::public_key_from_ed25519(&[1, 2, 3]).is_err());
}
