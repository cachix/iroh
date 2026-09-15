use std::collections::BTreeSet;

use iroh_base::TransportAddr;
use iroh_identity::{LocalIdentity, Registry, SignedContact};

#[test]
fn contact_verification_binds_identity_policy_freshness_and_addresses() {
    let registry = Registry::builtins(vec![1, 2]).unwrap();
    let identity = LocalIdentity::generate_ml_dsa65(&registry).unwrap();
    let stranger = LocalIdentity::generate_ml_dsa65(&registry).unwrap();
    let addresses = BTreeSet::from([TransportAddr::Ip("127.0.0.1:1234".parse().unwrap())]);
    let record = SignedContact::sign(&identity, 5, 1000, 1100, addresses.clone()).unwrap();
    let verified =
        SignedContact::verify(record.as_bytes(), identity.id(), &registry, 1001).unwrap();
    assert_eq!(verified.addresses(), &addresses);
    assert_eq!(verified.sequence(), 5);
    assert!(SignedContact::verify(record.as_bytes(), stranger.id(), &registry, 1001).is_err());
    assert!(
        SignedContact::verify(
            record.as_bytes(),
            identity.id(),
            &Registry::builtins(vec![1]).unwrap(),
            1001
        )
        .is_err()
    );
    for now in [900, 1100, 1200] {
        assert!(SignedContact::verify(record.as_bytes(), identity.id(), &registry, now).is_err());
    }
    let mut changed = record.as_bytes().to_vec();
    let last = changed.len() - 1;
    changed[last] ^= 1;
    assert!(SignedContact::verify(&changed, identity.id(), &registry, 1001).is_err());
    for length in [0, 1, 20, 100, record.as_bytes().len() - 1] {
        assert!(
            SignedContact::verify(&record.as_bytes()[..length], identity.id(), &registry, 1001)
                .is_err()
        );
    }
    assert!(SignedContact::sign(&identity, 6, 1000, 5000, addresses).is_err());
}
