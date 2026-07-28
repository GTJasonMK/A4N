use a4n_capabilities::{CapabilityClient, HttpExportClient};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let base_url = args.next().expect("base URL argument");
    let expected_compatibility = match args.next().as_deref() {
        Some("advertised") => true,
        Some("legacy") => false,
        _ => panic!("expected compatibility mode: advertised or legacy"),
    };
    let health = HttpExportClient::new(base_url)
        .health()
        .await
        .unwrap_or_else(|error| panic!("health probe failed: {error:?}"));
    assert_eq!(health.service, "export");
    assert_eq!(health.version.protocol_version, 2);
    let json = serde_json::to_value(health).expect("health response should serialize");
    assert_eq!(
        json["version"].get("compatibility").is_some(),
        expected_compatibility,
        "unexpected health compatibility envelope: {json}"
    );
}
