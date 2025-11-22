// Integration tests
use devobox::domain::{ContainerSpec, Database};

#[test]
fn test_database_to_spec_conversion() {
    let db = Database {
        name: "test_postgres".to_string(),
        image: "postgres:15".to_string(),
        ports: vec!["5432:5432".to_string()],
        env: vec!["POSTGRES_PASSWORD=secret".to_string()],
        volumes: vec!["/data:/var/lib/postgresql".to_string()],
    };

    let spec = db.to_spec();

    assert_eq!(spec.name, "test_postgres");
    assert_eq!(spec.image, "postgres:15");
    assert_eq!(spec.ports, &["5432:5432".to_string()]);
    assert_eq!(spec.env, &["POSTGRES_PASSWORD=secret".to_string()]);
    assert_eq!(spec.volumes, &["/data:/var/lib/postgresql".to_string()]);
}

#[test]
fn test_container_spec_creation() {
    let spec = ContainerSpec {
        name: "test-container",
        image: "alpine:latest",
        ports: &[],
        env: &[],
        network: Some("bridge"),
        userns: Some("keep-id"),
        security_opt: None,
        workdir: Some("/app"),
        volumes: &[],
        extra_args: &["--rm"],
    };

    assert_eq!(spec.name, "test-container");
    assert_eq!(spec.image, "alpine:latest");
    assert_eq!(spec.network, Some("bridge"));
    assert_eq!(spec.workdir, Some("/app"));
}
