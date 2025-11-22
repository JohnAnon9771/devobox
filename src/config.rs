use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Database {
    pub name: String,
    pub image: String,
    #[serde(default)]
    pub ports: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub volumes: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum DatabaseDocument {
    Root { databases: Vec<Database> },
    List(Vec<Database>),
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

    for file in ["Containerfile", "databases.yml"] {
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

fn parse_databases(content: &str, path: &Path) -> Result<Vec<Database>> {
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let doc: DatabaseDocument =
        serde_yaml::from_str(content).with_context(|| format!("parse de {:?}", path))?;

    let mut databases = match doc {
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

    Ok(databases.drain(..).collect())
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
}
