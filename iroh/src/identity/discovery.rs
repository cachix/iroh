use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use super::{Error, PeerId, SignedContact};

/// Future returned by a typed address lookup backend.
pub type LookupFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, Error>> + Send + 'a>>;

/// Storage for signed contacts. The endpoint verifies every retrieved record.
pub trait AddressLookup: std::fmt::Debug + Send + Sync + 'static {
    /// Store an already signed contact record.
    fn publish(&self, contact: SignedContact) -> LookupFuture<'_, ()>;
    /// Retrieve a bounded wire record for this exact identity.
    fn resolve(&self, peer: PeerId) -> LookupFuture<'_, Vec<u8>>;
}

/// HTTP discovery backed by an explicitly enabled identity relay service.
#[derive(Clone, Debug)]
pub struct HttpDiscovery {
    base: url::Url,
    client: reqwest::Client,
}

impl HttpDiscovery {
    /// Configure a discovery origin and CA policy. Redirects are disabled.
    ///
    /// HTTPS uses the default aws-lc-rs provider; endpoints configured with a
    /// custom [`crate::endpoint::Builder::crypto_provider`] should pass the
    /// same provider to [`Self::with_provider`].
    pub fn new(base: url::Url, tls: crate::tls::CaTlsConfig) -> Result<Self, Error> {
        Self::with_provider(
            base,
            tls,
            Arc::new(rustls::crypto::aws_lc_rs::default_provider()),
        )
    }

    /// Configure a discovery origin, CA policy and the rustls provider for HTTPS.
    pub fn with_provider(
        base: url::Url,
        tls: crate::tls::CaTlsConfig,
        provider: Arc<rustls::crypto::CryptoProvider>,
    ) -> Result<Self, Error> {
        if !matches!(base.scheme(), "http" | "https") {
            return Err(Error::Protocol("invalid discovery origin"));
        }
        let config = tls.client_config(provider)?;
        let client = reqwest::Client::builder()
            .use_preconfigured_tls(config)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(io_error)?;
        Ok(Self { base, client })
    }

    fn url(&self, peer: PeerId) -> url::Url {
        let mut url = self.base.clone();
        url.set_path(&format!("{}{peer}", iroh_relay::identity::DISCOVERY_PATH));
        url.set_query(None);
        url.set_fragment(None);
        url
    }
}

fn io_error(error: reqwest::Error) -> Error {
    Error::Io(std::io::Error::other(error))
}

impl AddressLookup for HttpDiscovery {
    fn publish(&self, contact: SignedContact) -> LookupFuture<'_, ()> {
        Box::pin(async move {
            self.client
                .put(self.url(contact.id()))
                .body(contact.as_bytes().to_vec())
                .send()
                .await
                .map_err(io_error)?
                .error_for_status()
                .map_err(io_error)?;
            Ok(())
        })
    }

    fn resolve(&self, peer: PeerId) -> LookupFuture<'_, Vec<u8>> {
        Box::pin(async move {
            let mut response = self
                .client
                .get(self.url(peer))
                .send()
                .await
                .map_err(io_error)?
                .error_for_status()
                .map_err(io_error)?;
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(io_error)? {
                if bytes.len() + chunk.len() > iroh_identity::MAX_CONTACT_SIZE {
                    return Err(Error::Protocol("discovery response too large"));
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(bytes)
        })
    }
}
