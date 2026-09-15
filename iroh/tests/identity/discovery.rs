use std::{sync::Arc, time::Duration};

use iroh::identity::{HttpDiscovery, LocalIdentity, Registry, RemotePolicy};
use iroh::{Endpoint, RelayMap, RelayMode, endpoint::presets::Empty};

#[tokio::test]
async fn discovery_and_upgrade_share_source_limits_ignoring_forwarded_headers() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let registry = Arc::new(Registry::builtins(vec![2]).unwrap());
        let (relay, url) = super::relay::relay(registry).await;
        let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(rustls::RootCertStore::empty())
        .with_no_client_auth();
        let client = reqwest::Client::builder()
            .use_preconfigured_tls(tls)
            .build()
            .unwrap();
        let contact = format!(
            "{}{}{}",
            url.as_str().trim_end_matches('/'),
            iroh_relay::identity::DISCOVERY_PATH,
            iroh::identity::PeerId::V1([8; 48])
        );
        for n in 0..120 {
            let response = client
                .get(&contact)
                .header("X-Forwarded-For", format!("192.0.2.{n}"))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
        }
        assert_eq!(
            client.get(&contact).send().await.unwrap().status(),
            reqwest::StatusCode::TOO_MANY_REQUESTS
        );
        let upgrade = client
            .get(format!(
                "{}{}",
                url.as_str().trim_end_matches('/'),
                iroh_relay::identity::PATH
            ))
            .header("Upgrade", "websocket")
            .header("Connection", "upgrade")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Protocol", iroh_relay::identity::PROTOCOL)
            .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
            .send()
            .await
            .unwrap();
        assert_eq!(upgrade.status(), reqwest::StatusCode::TOO_MANY_REQUESTS);
        relay.shutdown().await.unwrap();
    })
    .await
    .expect("source admission test timeout");
}

#[tokio::test]
async fn pq_discovery_dials_by_identity_across_different_home_relays() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let registry = Arc::new(Registry::builtins(vec![1, 2]).unwrap());
        let (relay_a, url_a) = super::relay::relay(registry.clone()).await;
        let (relay_b, url_b) = super::relay::relay(registry.clone()).await;
        let lookup = Arc::new(
            HttpDiscovery::new(url_a.as_str().parse().unwrap(), Default::default()).unwrap(),
        );
        let mut endpoints = Vec::new();
        for url in [url_a, url_b] {
            let identity = LocalIdentity::generate_ml_dsa65(&registry).unwrap();
            let endpoint = Endpoint::builder(Empty)
                .clear_ip_transports()
                .credentials(identity, registry.clone())
                .remote_policy(RemotePolicy::new([2]))
                .relay_mode(RelayMode::Custom(RelayMap::from_iter([url])))
                .address_lookup(lookup.clone())
                .alpns(vec![b"discovery-test/1".to_vec()])
                .bind()
                .await
                .unwrap();
            endpoint.publish(1, Duration::from_secs(120)).await.unwrap();
            endpoints.push(endpoint);
        }
        let (outgoing, incoming) = tokio::join!(
            endpoints[0].connect(endpoints[1].id(), b"discovery-test/1"),
            endpoints[1].accept()
        );
        let outgoing = outgoing.unwrap();
        let incoming = incoming.unwrap();
        assert_eq!(outgoing.remote_id(), endpoints[1].id());
        assert_eq!(incoming.remote_id(), endpoints[0].id());
        let mut stream = outgoing.open_uni().await.unwrap();
        stream.write_all(b"discovered PQ peer").await.unwrap();
        stream.finish().unwrap();
        assert_eq!(
            incoming
                .accept_uni()
                .await
                .unwrap()
                .read_to_end(64)
                .await
                .unwrap(),
            b"discovered PQ peer"
        );
        // A valid older record still cannot replace a currently stored sequence.
        endpoints[1]
            .publish(2, Duration::from_secs(120))
            .await
            .unwrap();
        assert!(
            endpoints[1]
                .publish(1, Duration::from_secs(120))
                .await
                .is_err()
        );
        for endpoint in endpoints {
            endpoint.close().await;
        }
        relay_a.shutdown().await.unwrap();
        relay_b.shutdown().await.unwrap();
    })
    .await
    .expect("discovery test timeout");
}
