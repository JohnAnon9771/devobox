use anyhow::Result;
use devobox::domain::traits::ContainerHealthStatus;
use devobox::domain::{ContainerState, Service, ServiceKind};
use devobox::services::{ContainerService, Orchestrator, SystemService};
use devobox::test_support::MockRuntime;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn create_orchestrator() -> (Orchestrator, Arc<MockRuntime>) {
    let mock = Arc::new(MockRuntime::new());
    let container_service = Arc::new(ContainerService::new(mock.clone()));
    let system_service = Arc::new(SystemService::new(mock.clone()));
    let orchestrator = Orchestrator::new(container_service, system_service);
    (orchestrator, mock)
}

#[test]
fn test_stress_healthcheck_timeout() -> Result<()> {
    // Tests if the orchestrator correctly fails after N retries
    // and doesn't hang indefinitely consuming CPU
    let (orchestrator, mock) = create_orchestrator();

    let svc = Service {
        name: "slow_service".to_string(),
        image: "img".to_string(),
        kind: ServiceKind::Generic,
        ports: vec![],
        env: vec![],
        volumes: vec![],
        healthcheck_command: Some("cmd".into()),
        healthcheck_interval: Some("10ms".into()), // Fast interval for test speed
        healthcheck_timeout: Some("10ms".into()),
        healthcheck_retries: Some(3),
    };

    mock.add_container("slow_service", ContainerState::Stopped);
    // Always return Unhealthy
    mock.set_health_status("slow_service", ContainerHealthStatus::Unhealthy);

    let start = Instant::now();
    let result = orchestrator.start_all(&[svc]);
    let duration = start.elapsed();

    assert!(result.is_err());
    // Should run for roughly: 3 retries * 10ms interval = ~30ms
    // Giving a generous upper bound for thread scheduling overhead
    assert!(
        duration < Duration::from_secs(1),
        "Healthcheck loop hung too long"
    );
    assert!(
        duration > Duration::from_millis(20),
        "Healthcheck loop failed too fast"
    );

    Ok(())
}

#[test]
fn test_resilience_flaky_service() -> Result<()> {
    // BUG HUNT: Current implementation fails immediately on first Unhealthy status?
    // A robust system should retry if it sees 'Unhealthy' until retries run out.
    let (orchestrator, mock) = create_orchestrator();

    let svc = Service {
        name: "flaky".to_string(),
        image: "img".to_string(),
        kind: ServiceKind::Generic,
        ports: vec![],
        env: vec![],
        volumes: vec![],
        healthcheck_command: Some("cmd".into()),
        healthcheck_interval: Some("10ms".into()),
        healthcheck_timeout: Some("10ms".into()),
        healthcheck_retries: Some(5),
    };

    mock.add_container("flaky", ContainerState::Stopped);
    mock.set_health_status("flaky", ContainerHealthStatus::Unhealthy);

    // Simulate "becoming healthy" after a delay
    let mock_clone = mock.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(25));
        mock_clone.set_health_status("flaky", ContainerHealthStatus::Healthy);
    });

    let result = orchestrator.start_all(&[svc]);

    // If this fails, it means the orchestrator gave up on the first "Unhealthy" check
    // instead of consuming a retry.
    assert!(
        result.is_ok(),
        "Orchestrator should tolerate temporary unhealthy status"
    );

    Ok(())
}

#[test]
fn test_performance_serial_execution_bottleneck() -> Result<()> {
    // Test that parallel execution significantly reduces total startup time.
    // Start 3 services, each simulated to take 50ms to become healthy.
    // If serial, total time would be ~150ms.
    // If parallel, total time should be ~50ms (max of individual times) + overhead.

    let (orchestrator, mock) = create_orchestrator();

    let services: Vec<Service> = (0..3)
        .map(|i| Service {
            name: format!("svc_{}", i),
            image: "img".to_string(),
            kind: ServiceKind::Generic,
            ports: vec![],
            env: vec![],
            volumes: vec![],
            healthcheck_command: Some("cmd".into()),
            healthcheck_interval: Some("20ms".into()), // Each healthcheck check takes 20ms
            healthcheck_timeout: Some("20ms".into()),
            healthcheck_retries: Some(10), // Sufficient retries
        })
        .collect();

    for s in &services {
        mock.add_container(&s.name, ContainerState::Stopped);
        mock.set_health_status(&s.name, ContainerHealthStatus::Starting);
    }

    // Background thread makes all services healthy after 50ms
    let mock_clone = mock.clone();
    let svc_names: Vec<String> = services.iter().map(|s| s.name.clone()).collect();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50)); // After 50ms, all are healthy
        for name in svc_names {
            mock_clone.set_health_status(&name, ContainerHealthStatus::Healthy);
        }
    });

    let start = Instant::now();
    orchestrator.start_all(&services).unwrap();
    let duration = start.elapsed();

    // With parallel execution, total time should be close to 50ms (+ some overhead).
    // If it were serial, it would be 3 * 50ms = 150ms minimum (ignoring healthcheck interval/retries for simplicity).
    // Let's allow for overhead, but ensure it's much faster than serial.
    let expected_min_duration = Duration::from_millis(50);
    let expected_max_duration = Duration::from_millis(100); // Allow up to double the ideal parallel time for test stability

    assert!(
        duration >= expected_min_duration,
        "Parallel execution took too little time: {:?}",
        duration
    );
    assert!(
        duration < expected_max_duration,
        "Parallel execution took too long ({:?}), expected < {:?}",
        duration,
        expected_max_duration
    );

    Ok(())
}
