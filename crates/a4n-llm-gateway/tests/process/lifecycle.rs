use a4n_service_protocol::{LLM_SERVICE_NAME, ServiceErrorCode};

use super::support::{
    assert_gateway_health, assert_port_released, gateway_manager, unused_local_addr,
    wait_until_unavailable,
};

#[tokio::test]
async fn manager_starts_and_stops_independent_gateway_binary() {
    let addr = unused_local_addr();
    let manager = gateway_manager(addr);

    let process = manager
        .start_sidecar(LLM_SERVICE_NAME)
        .await
        .expect("manager should start the independent gateway binary");
    assert_eq!(process.service_name(), LLM_SERVICE_NAME);
    assert!(process.id().is_some_and(|pid| pid > 0));
    assert_gateway_health(&manager, addr).await;

    process
        .stop()
        .await
        .expect("manager-owned gateway should stop explicitly");
    let error = wait_until_unavailable(&manager).await;
    assert_eq!(
        error.protocol_error_code(),
        ServiceErrorCode::ServiceUnavailable
    );
    assert_port_released(addr);
}

#[tokio::test]
async fn dropping_manager_owned_handle_kills_gateway_and_releases_port() {
    let addr = unused_local_addr();
    let manager = gateway_manager(addr);

    let process = manager
        .start_sidecar(LLM_SERVICE_NAME)
        .await
        .expect("manager should start the independent gateway binary");
    assert!(process.id().is_some_and(|pid| pid > 0));
    assert_gateway_health(&manager, addr).await;

    drop(process);
    let error = wait_until_unavailable(&manager).await;
    assert_eq!(
        error.protocol_error_code(),
        ServiceErrorCode::ServiceUnavailable
    );
    assert_port_released(addr);
}
