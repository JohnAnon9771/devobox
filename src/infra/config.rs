use crate::domain::Database;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(untagged)]
enum DatabaseDocument {
    Root { databases: Vec<Database> },
    List(Vec<Database>),
}

#[derive(Deserialize, Debug)]
pub struct MiseConfig {
    pub tools: std::collections::HashMap<String, String>,
}

pub fn default_config_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/home/dev"))
        .join(".config/devobox")
}

pub fn databases_path(config_dir: &Path) -> PathBuf {
    config_dir.join("databases.yml")
}

pub fn ensure_config_dir(config_dir: &Path) -> Result<()> {
    fs::create_dir_all(config_dir).with_context(|| format!("criando {:?}", config_dir))
}

pub fn copy_template_if_missing(source_dir: &Path, target_dir: &Path) -> Result<()> {
    ensure_config_dir(target_dir)?;

    for file in ["Containerfile", "databases.yml", "mise.toml"] {
        let source = source_dir.join(file);
        let target = target_dir.join(file);

        if target.exists() {
            continue;
        }

        fs::copy(&source, &target)
            .with_context(|| format!("copiando template de {:?} para {:?}", source, target))?;
    }

    Ok(())
}

pub fn load_databases(config_dir: &Path) -> Result<Vec<Database>> {
    let path = databases_path(config_dir);

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&path).with_context(|| format!("lendo {:?}", path))?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    parse_databases(&content, &path)
}

pub fn load_mise_config(config_dir: &Path) -> Result<MiseConfig> {
    let path = config_dir.join("mise.toml");

    if !path.exists() {
        bail!("mise.toml não encontrado em {:?}", config_dir);
    }

    let content = fs::read_to_string(&path).with_context(|| format!("lendo {:?}", path))?;
    let config: MiseConfig =
        toml::from_str(&content).with_context(|| format!("parse de {:?}", path))?;

    Ok(config)
}

fn parse_databases(content: &str, path: &Path) -> Result<Vec<Database>> {
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let doc: DatabaseDocument =
        serde_yaml::from_str(content).with_context(|| format!("parse de {:?}", path))?;

    let databases = match doc {
        DatabaseDocument::Root { databases } => databases,
        DatabaseDocument::List(list) => list,
    };

    let mut names = HashSet::new();

    for (idx, db) in databases.iter().enumerate() {
        if db.name.trim().is_empty() {
            bail!("Entrada {} em {:?} sem 'name'", idx + 1, path);
        }

        if db.image.trim().is_empty() {
            bail!("Entrada {} em {:?} sem 'image'", idx + 1, path);
        }

        if !names.insert(db.name.clone()) {
            bail!(
                "Entrada {} em {:?} duplicou o nome '{}'",
                idx + 1,
                path,
                db.name
            );
        }
    }

    Ok(databases)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parses_root_databases_key() {
        let yaml = r#"
databases:
  - name: pg
    image: postgres:15
    ports: ["5432:5432"]
    env:
      - POSTGRES_PASSWORD=dev
    volumes:
      - /var/lib/postgresql/data
  - name: redis
    image: docker.io/redis:7
"#;

        let dbs = parse_databases(yaml, Path::new("databases.yml")).unwrap();
        assert_eq!(dbs.len(), 2);
        assert_eq!(dbs[0].name, "pg");
        assert_eq!(dbs[0].env, vec!["POSTGRES_PASSWORD=dev".to_string()]);
        assert_eq!(dbs[0].volumes, vec!["/var/lib/postgresql/data".to_string()]);
        assert_eq!(dbs[1].ports, Vec::<String>::new());
        assert!(dbs[1].volumes.is_empty());
    }

    #[test]
    fn parses_list_style() {
        let yaml = r#"
- name: pg
  image: postgres:15
  ports:
    - "5432:5432"
"#;

        let dbs = parse_databases(yaml, Path::new("databases.yml")).unwrap();
        assert_eq!(dbs.len(), 1);
        assert_eq!(dbs[0].ports, vec!["5432:5432".to_string()]);
    }

    #[test]
    fn rejects_duplicate_names() {
        let yaml = r#"
databases:
  - name: pg
    image: postgres:15
  - name: pg
    image: postgres:16
"#;

        let err = parse_databases(yaml, Path::new("databases.yml")).unwrap_err();
        assert!(err.to_string().contains("duplicou o nome"));
    }

    #[test]
    fn rejects_missing_required_fields() {
        let yaml = r#"
- name: ""
  image: postgres:15
"#;

        let err = parse_databases(yaml, Path::new("databases.yml")).unwrap_err();
        assert!(err.to_string().contains("sem 'name'"));
    }

    #[test]
    fn empty_file_is_allowed() {
        let parsed = parse_databases("   \n", Path::new("databases.yml"));
        assert_eq!(parsed.unwrap().len(), 0);
    }

    #[test]
    fn rejects_missing_image() {
        let yaml = r#"
- name: pg
  image: ""
"#;

        let err = parse_databases(yaml, Path::new("databases.yml")).unwrap_err();
        assert!(err.to_string().contains("sem 'image'"));
    }

    #[test]
    fn parses_database_with_minimal_fields() {
        let yaml = r#"
- name: minimal
  image: minimal:latest
"#;

        let dbs = parse_databases(yaml, Path::new("databases.yml")).unwrap();
        assert_eq!(dbs.len(), 1);
        assert_eq!(dbs[0].name, "minimal");
        assert_eq!(dbs[0].image, "minimal:latest");
        assert!(dbs[0].ports.is_empty());
        assert!(dbs[0].env.is_empty());
        assert!(dbs[0].volumes.is_empty());
    }

    #[test]
    fn parses_multiple_databases() {
        let yaml = r#"
databases:
  - name: db1
    image: postgres:15
  - name: db2
    image: mysql:8
  - name: db3
    image: redis:7
"#;

        let dbs = parse_databases(yaml, Path::new("databases.yml")).unwrap();
        assert_eq!(dbs.len(), 3);
        assert_eq!(dbs[0].name, "db1");
        assert_eq!(dbs[1].name, "db2");
        assert_eq!(dbs[2].name, "db3");
    }

    #[test]
    fn preserves_database_order() {
        let yaml = r#"
- name: first
  image: first:1
- name: second
  image: second:2
- name: third
  image: third:3
"#;

        let dbs = parse_databases(yaml, Path::new("databases.yml")).unwrap();
        assert_eq!(dbs[0].name, "first");
        assert_eq!(dbs[1].name, "second");
        assert_eq!(dbs[2].name, "third");
    }
    #[test]
    fn copies_mise_toml() {
        let temp_dir = std::env::temp_dir().join("devobox_test_mise");
        let source_dir = temp_dir.join("source");
        let target_dir = temp_dir.join("target");

        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("Containerfile"), "").unwrap();
        fs::write(source_dir.join("databases.yml"), "").unwrap();
        fs::write(source_dir.join("mise.toml"), "content").unwrap();

        copy_template_if_missing(&source_dir, &target_dir).unwrap();

        assert!(target_dir.join("mise.toml").exists());
        assert_eq!(
            fs::read_to_string(target_dir.join("mise.toml")).unwrap(),
            "content"
        );

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn parses_mise_toml() {
        let toml = r#"
[tools]
ruby = "3.2"
node = "20"
"#;
        let temp_dir = std::env::temp_dir().join("devobox_test_mise_parse");
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(temp_dir.join("mise.toml"), toml).unwrap();

        let config = load_mise_config(&temp_dir).unwrap();
        assert_eq!(config.tools.get("ruby").unwrap(), "3.2");
        assert_eq!(config.tools.get("node").unwrap(), "20");

        fs::remove_dir_all(temp_dir).unwrap();
    }
}
