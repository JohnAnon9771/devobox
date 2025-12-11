use anyhow::Result;
use devobox::cli::runtime::Runtime;
use devobox::domain::ContainerState;
use devobox::test_support::MockRuntime;
use std::fs;
use std::sync::Arc;

#[test]
fn test_workflow_up_and_down() -> Result<()> {
    // 1. Setup Temp Config
    let temp_dir = tempfile::tempdir()?;
    let config_path = temp_dir.path();

    // Write devobox.toml
    let devobox_toml = r#"
[container]
name = "devobox-test"
workdir = "/home/dev"

[build]
image_name = "devobox:test"

[services.postgres]
image = "postgres:15"
type = "database"
ports = ["5432:5432"]
"#;
    fs::write(config_path.join("devobox.toml"), devobox_toml)?;

    // 2. Setup Mock Runtime
    let mock = Arc::new(MockRuntime::new());
    // Simulate that 'devobox build' has already run (container exists but is stopped)
    mock.add_container("devobox-test", ContainerState::Stopped);

    // 3. Initialize Runtime
    // Note: Runtime::with_runtime will try to resolve services.
    let runtime = Runtime::with_runtime(config_path, mock.clone())?;

    // 4. Simulate "devobox up" behavior
    // "up" basically starts services and ensures dev container is running
    println!("--- Simulating devobox up ---");
    runtime.start_services_by_filter(None)?;
    runtime.ensure_dev_container()?;

    // 5. Assertions (Up)
    assert_eq!(
        mock.get_state("postgres"),
        Some(ContainerState::Running),
        "Postgres should be running"
    );
    assert_eq!(
        mock.get_state("devobox-test"),
        Some(ContainerState::Running),
        "Devobox container should be running"
    );

    let commands = mock.get_commands();
    // Verify creation happens
    assert!(
        commands.contains(&"create:postgres".to_string()),
        "Should have created postgres"
    );
    // devobox-test was pre-seeded as Stopped, so it shouldn't be re-created, only started.
    // assert!(
    //    commands.contains(&"create:devobox-test".to_string()),
    //    "Should have created devobox-test"
    // );

    // Verify start happens
    assert!(
        commands.contains(&"start:postgres".to_string()),
        "Should have started postgres"
    );
    assert!(
        commands.contains(&"start:devobox-test".to_string()),
        "Should have started devobox-test"
    );

    // 6. Simulate "devobox down" behavior
    println!("--- Simulating devobox down ---");
    runtime.stop_all_containers()?;

    // 7. Assertions (Down)
    assert_eq!(
        mock.get_state("postgres"),
        Some(ContainerState::Stopped),
        "Postgres should be stopped"
    );
    assert_eq!(
        mock.get_state("devobox-test"),
        Some(ContainerState::Stopped),
        "Devobox container should be stopped"
    );

    let commands_after = mock.get_commands();
    assert!(
        commands_after.contains(&"stop:postgres".to_string()),
        "Should have stopped postgres"
    );
    assert!(
        commands_after.contains(&"stop:devobox-test".to_string()),
        "Should have stopped devobox-test"
    );

    Ok(())
}

#[test]
fn test_workflow_status() -> Result<()> {
    // 1. Setup Temp Config
    let temp_dir = tempfile::tempdir()?;
    let config_path = temp_dir.path();

    fs::write(
        config_path.join("devobox.toml"),
        r#"[container]
name = "devobox-test"
workdir = "/home/dev"
[build]
image_name = "devobox:test"
"#,
    )?;

    // 2. Setup Mock Runtime
    let mock = Arc::new(MockRuntime::new());
    mock.add_container("devobox-test", ContainerState::Running);

    // 3. Initialize Runtime
    let runtime = Runtime::with_runtime(config_path, mock.clone())?;

    // 4. Run status (just ensures it doesn't panic and calls get_state)
    // We capture stdout in a real scenario, but here we just check mock interactions
    runtime.status()?;

    let commands = mock.get_commands();
    // It should check status of devobox-test
    // Note: implementation of get_status calls get_container underneath?
    // Let's check ContainerService::get_status implementation or MockRuntime
    // MockRuntime records "get_container:name"
    assert!(
        commands
            .iter()
            .any(|c| c.contains("get_container:devobox-test"))
    );

    Ok(())
}
