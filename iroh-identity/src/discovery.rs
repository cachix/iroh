use std::collections::BTreeSet;

use iroh_base::TransportAddr;

use crate::{Error, LocalIdentity, PeerId, Registry};

const MAGIC: &[u8] = b"iroh signed contact v1\0";
/// Maximum signed contact wire size, including a PQ public key and signature.
pub const MAX_CONTACT_SIZE: usize = 32768;

/// A bounded, signed discovery record whose addresses remain routing hints.
///
/// Records expire within an hour. Applications supply a monotonically increasing
/// sequence and retain it across restarts; resolvers reject observed rollbacks.
#[derive(Clone, Debug)]
pub struct SignedContact {
    id: PeerId,
    issued: u64,
    expires: u64,
    sequence: u64,
    addresses: BTreeSet<TransportAddr>,
    wire: Vec<u8>,
}

impl SignedContact {
    /// Sign a record valid at Unix seconds `issued` until `expires` (at most one hour).
    pub fn sign(
        identity: &LocalIdentity,
        sequence: u64,
        issued: u64,
        expires: u64,
        addresses: BTreeSet<TransportAddr>,
    ) -> Result<Self, Error> {
        validate(issued, expires, issued, &addresses)?;
        let public = identity.public_key();
        let mut wire = MAGIC.to_vec();
        append(&mut wire, public.as_ref())?;
        wire.extend_from_slice(&issued.to_be_bytes());
        wire.extend_from_slice(&expires.to_be_bytes());
        wire.extend_from_slice(&sequence.to_be_bytes());
        wire.extend_from_slice(&(addresses.len() as u16).to_be_bytes());
        for address in &addresses {
            append(&mut wire, address.to_string().as_bytes())?;
        }
        wire.extend_from_slice(&identity.sign(&wire)?);
        if wire.len() > MAX_CONTACT_SIZE {
            return Err(Error::Protocol("contact too large"));
        }
        Ok(Self {
            id: identity.id(),
            sequence,
            issued,
            expires,
            addresses,
            wire,
        })
    }

    /// Verify exact identity, remote policy, canonical encoding, signature and freshness.
    pub fn verify(
        wire: &[u8],
        expected: PeerId,
        registry: &Registry,
        now: u64,
    ) -> Result<Self, Error> {
        if wire.len() > MAX_CONTACT_SIZE || !wire.starts_with(MAGIC) {
            return Err(Error::Encoding);
        }
        let mut rest = &wire[MAGIC.len()..];
        let public = field(&mut rest)?;
        if public.len() > 8192 {
            return Err(Error::PublicIdentity);
        }
        let issued = number(&mut rest)?;
        let expires = number(&mut rest)?;
        let sequence = number(&mut rest)?;
        let count = u16::from_be_bytes(take(&mut rest, 2)?.try_into().expect("checked length"));
        if count > 32 {
            return Err(Error::Protocol("too many contact addresses"));
        }
        let mut addresses = BTreeSet::new();
        let mut previous = None;
        for _ in 0..count {
            let text = std::str::from_utf8(field(&mut rest)?).map_err(|_| Error::Encoding)?;
            let address = if let Some(ip) = text.strip_prefix("ip:") {
                TransportAddr::Ip(ip.parse().map_err(|_| Error::Encoding)?)
            } else if let Some(relay) = text.strip_prefix("relay:") {
                TransportAddr::Relay(relay.parse().map_err(|_| Error::Encoding)?)
            } else {
                return Err(Error::Encoding);
            };
            if address.to_string() != text || previous.as_ref().is_some_and(|p| p >= &address) {
                return Err(Error::Encoding);
            }
            previous = Some(address.clone());
            addresses.insert(address);
        }
        validate(issued, expires, now, &addresses)?;
        let id = registry.verify_proof(public, &wire[..wire.len() - rest.len()], rest)?;
        if id != expected {
            return Err(Error::IdentityMismatch);
        }
        Ok(Self {
            id,
            issued,
            expires,
            sequence,
            addresses,
            wire: wire.to_vec(),
        })
    }

    /// Identity authenticated by the record signature.
    pub fn id(&self) -> PeerId {
        self.id
    }
    /// Monotonic publisher sequence used for rollback rejection.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    /// Unix timestamp when this record becomes valid.
    pub fn issued(&self) -> u64 {
        self.issued
    }
    /// Unix timestamp after which this record must be discarded.
    pub fn expires(&self) -> u64 {
        self.expires
    }
    /// Authenticated routing hints; TLS must still authenticate the connection.
    pub fn addresses(&self) -> &BTreeSet<TransportAddr> {
        &self.addresses
    }
    /// Canonical wire encoding including the signature.
    pub fn as_bytes(&self) -> &[u8] {
        &self.wire
    }
}

fn validate(
    issued: u64,
    expires: u64,
    now: u64,
    addresses: &BTreeSet<TransportAddr>,
) -> Result<(), Error> {
    if expires <= issued
        || expires - issued > 3600
        || issued > now.saturating_add(30)
        || expires <= now
    {
        return Err(Error::Protocol("contact validity window"));
    }
    if addresses.is_empty() || addresses.len() > 32 {
        return Err(Error::Protocol("invalid contact addresses"));
    }
    for address in addresses {
        match address {
            TransportAddr::Ip(ip)
                if !ip.ip().is_unspecified() && !ip.ip().is_multicast() && ip.port() != 0 => {}
            TransportAddr::Relay(url)
                if matches!(url.scheme(), "https" | "http") && url.as_str().len() <= 2048 => {}
            _ => return Err(Error::Protocol("unsupported contact address")),
        }
    }
    Ok(())
}

fn append(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), Error> {
    let len: u16 = bytes.len().try_into().map_err(|_| Error::Encoding)?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(bytes);
    Ok(())
}
fn take<'a>(rest: &mut &'a [u8], count: usize) -> Result<&'a [u8], Error> {
    if rest.len() < count {
        return Err(Error::Encoding);
    }
    let (value, tail) = rest.split_at(count);
    *rest = tail;
    Ok(value)
}
fn field<'a>(rest: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    let len = u16::from_be_bytes(take(rest, 2)?.try_into().expect("checked length")) as usize;
    take(rest, len)
}
fn number(rest: &mut &[u8]) -> Result<u64, Error> {
    Ok(u64::from_be_bytes(
        take(rest, 8)?.try_into().expect("checked length"),
    ))
}
