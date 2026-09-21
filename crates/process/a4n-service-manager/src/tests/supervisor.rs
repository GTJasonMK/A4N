use super::*;

#[tokio::test]
#[ignore = "requires A4N_DEMO_SIDECAR_BIN"]
async fn supervisor_restarts_once_and_does_not_replay_in_flight_request() {
    let binary = demo_sidecar_binary_from_env();
    let addr = unused_local_addr();
    let base_url = format!("http://{addr}");
    let counter = std::env::temp_dir().join(format!(
        "a4n-sidecar-counter-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let token = "demo-supervisor-token";
    let mut config = ManagedServiceConfig::sidecar("demo", binary, base_url.clone());
    config
        .env
        .insert("A4N_DEMO_SIDECAR_ADDR".to_string(), addr.clone());
    config
        .env
        .insert(SIDECAR_AUTH_TOKEN_ENV.to_string(), token.to_string());
    config.env.insert(
        "A4N_DEMO_SIDECAR_COUNTER_FILE".to_string(),
        counter.display().to_string(),
    );
    config.restart_policy = RestartPolicy {
        max_restarts: 2,
        initial_backoff_ms: 20,
        max_backoff_ms: 40,
    };
    let manager = ServiceManager::new(vec![config]).unwrap();
    let process = manager.start_sidecar("demo").await.unwrap();
    let initial_pid = manager
        .process_status("demo")
        .await
        .unwrap()
        .and_then(|status| status.pid);
    let client = reqwest::Client::new();

    let in_flight = tokio::spawn({
        let client = client.clone();
        let base_url = base_url.clone();
        async move {
            client
                .post(base_url.clone() + "/counted-slow")
                .header(a4n_service_protocol::SIDECAR_AUTH_HEADER, token)
                .send()
                .await
        }
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    let _ = client
        .post(base_url + "/crash")
        .header(a4n_service_protocol::SIDECAR_AUTH_HEADER, token)
        .send()
        .await;
    let response = in_flight.await.unwrap();
    assert!(
        response.is_err(),
        "crashed in-flight request must fail explicitly"
    );

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(status) = manager.process_status("demo").await.unwrap()
                && status.state == ManagedSidecarState::Running
                && status.pid != initial_pid
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("supervisor should restart the demo sidecar");

    let lines = std::fs::read_to_string(&counter)
        .expect("in-flight request should leave one durable counter entry");
    assert_eq!(
        lines.lines().count(),
        1,
        "manager must not replay the request"
    );
    process.stop().await.unwrap();
    let _ = std::fs::remove_file(counter);
}

#[tokio::test]
#[ignore = "requires A4N_DEMO_SIDECAR_BIN"]
async fn supervisor_retries_transient_stop_failure_without_losing_child() {
    let binary = demo_sidecar_binary_from_env();
    let addr = unused_local_addr();
    let token_env = format!("A4N_TEST_STOP_TOKEN_{}", std::process::id());
    unsafe {
        std::env::remove_var(&token_env);
    }
    let mut config = ManagedServiceConfig::sidecar("demo", binary, format!("http://{addr}"));
    config.env.insert("A4N_DEMO_SIDECAR_ADDR".to_string(), addr);
    config.remote_security.auth_token_env = Some(token_env.clone());

    let manager = ServiceManager::new(vec![config]).unwrap();
    let mut process = manager.start_sidecar("demo").await.unwrap();
    let first_error = process
        .stop_in_place()
        .await
        .expect_err("missing stop credential should leave the supervisor retryable");
    assert!(
        first_error
            .to_string()
            .contains("credential environment is unavailable")
    );
    assert_eq!(
        manager
            .process_status("demo")
            .await
            .unwrap()
            .expect("failed stop should retain process status")
            .state,
        ManagedSidecarState::Stopping
    );

    unsafe {
        std::env::set_var(&token_env, "retry-stop-token");
    }
    process
        .stop_in_place()
        .await
        .expect("the second stop must retry the retained supervisor");
    assert_eq!(
        manager
            .process_status("demo")
            .await
            .unwrap()
            .expect("stopped process status should remain observable")
            .state,
        ManagedSidecarState::Stopped
    );
    unsafe {
        std::env::remove_var(token_env);
    }
}

#[tokio::test]
#[ignore = "requires A4N_DEMO_SIDECAR_BIN"]
async fn supervisor_opens_circuit_after_restart_budget_is_exhausted() {
    let binary = demo_sidecar_binary_from_env();
    let addr = unused_local_addr();
    let base_url = format!("http://{addr}");
    let token = "demo-circuit-token";
    let mut config = ManagedServiceConfig::sidecar("demo", binary, base_url.clone());
    config.env.insert("A4N_DEMO_SIDECAR_ADDR".to_string(), addr);
    config
        .env
        .insert(SIDECAR_AUTH_TOKEN_ENV.to_string(), token.to_string());
    config.restart_policy = RestartPolicy {
        max_restarts: 1,
        initial_backoff_ms: 10,
        max_backoff_ms: 10,
    };
    let manager = ServiceManager::new(vec![config]).unwrap();
    let process = manager.start_sidecar("demo").await.unwrap();
    let initial_pid = manager
        .process_status("demo")
        .await
        .unwrap()
        .and_then(|status| status.pid);
    let client = reqwest::Client::new();

    let _ = client
        .post(base_url.clone() + "/crash")
        .header(a4n_service_protocol::SIDECAR_AUTH_HEADER, token)
        .send()
        .await;
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if manager
                .process_status("demo")
                .await
                .unwrap()
                .is_some_and(|status| {
                    status.state == ManagedSidecarState::Running && status.pid != initial_pid
                })
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("first restart should become healthy");
    let _ = client
        .post(base_url + "/crash")
        .header(a4n_service_protocol::SIDECAR_AUTH_HEADER, token)
        .send()
        .await;

    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if manager
                .process_status("demo")
                .await
                .unwrap()
                .is_some_and(|status| status.state == ManagedSidecarState::CircuitOpen)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("restart budget should open the circuit");

    let mut process = process;
    let error = process
        .stop_in_place()
        .await
        .expect_err("stopping a circuit-open supervisor must expose its failure");
    assert!(error.to_string().contains("restart circuit opened"));
    let retry = process
        .stop_in_place()
        .await
        .expect_err("failed stop must remain an explicit retry failure");
    assert!(retry.to_string().contains("previous stop attempt failed"));
}

#[tokio::test]
#[ignore = "requires A4N_DEMO_SIDECAR_BIN"]
async fn two_managers_own_distinct_sidecars_and_stop_independently() {
    let binary = demo_sidecar_binary_from_env();

    let first_addr = unused_local_addr();
    let mut first_config =
        ManagedServiceConfig::sidecar("demo", binary.clone(), format!("http://{first_addr}"));
    first_config
        .env
        .insert("A4N_DEMO_SIDECAR_ADDR".to_string(), first_addr.clone());
    first_config.env.insert(
        SIDECAR_AUTH_TOKEN_ENV.to_string(),
        "first-manager-token".to_string(),
    );

    let first_manager = ServiceManager::new(vec![first_config]).unwrap();
    let first_process = first_manager.start_sidecar("demo").await.unwrap();

    let second_addr = unused_local_addr();
    assert_ne!(first_addr, second_addr);
    let mut second_config =
        ManagedServiceConfig::sidecar("demo", binary, format!("http://{second_addr}"));
    second_config
        .env
        .insert("A4N_DEMO_SIDECAR_ADDR".to_string(), second_addr.clone());
    second_config.env.insert(
        SIDECAR_AUTH_TOKEN_ENV.to_string(),
        "second-manager-token".to_string(),
    );

    let second_manager = ServiceManager::new(vec![second_config]).unwrap();
    let second_process = second_manager.start_sidecar("demo").await.unwrap();

    let first_pid = first_manager
        .process_status("demo")
        .await
        .unwrap()
        .and_then(|status| status.pid);
    let second_pid = second_manager
        .process_status("demo")
        .await
        .unwrap()
        .and_then(|status| status.pid);
    assert_ne!(first_pid, second_pid);
    assert_ne!(
        first_manager.auth_token("demo").unwrap(),
        second_manager.auth_token("demo").unwrap()
    );
    assert_eq!(first_manager.health("demo").await.unwrap().service, "demo");
    assert_eq!(second_manager.health("demo").await.unwrap().service, "demo");

    first_process.stop().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(first_manager.health("demo").await.is_err());
    assert_eq!(second_manager.health("demo").await.unwrap().service, "demo");
    assert_port_released(&first_addr);

    second_process.stop().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(second_manager.health("demo").await.is_err());
    assert_port_released(&second_addr);
}
