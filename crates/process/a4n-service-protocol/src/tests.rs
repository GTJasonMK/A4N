mod events_artifacts;
mod health;
mod request_validation;
mod sanitization;
mod strict_contracts;
mod support;

#[test]
fn service_socket_addr_has_one_strict_parser() {
    assert_eq!(
        super::parse_service_socket_addr("A4N_SERVICE_ADDR", " 127.0.0.1:47641 ").unwrap(),
        "127.0.0.1:47641".parse().unwrap()
    );
    assert!(super::parse_service_socket_addr("A4N_SERVICE_ADDR", " ").is_err());
    let error = super::parse_service_socket_addr("A4N_SERVICE_ADDR", "token=sidecar-secret")
        .expect_err("invalid address must fail");
    assert!(error.contains("A4N_SERVICE_ADDR 必须是有效监听地址"));
    assert!(!error.contains("sidecar-secret"));
}
