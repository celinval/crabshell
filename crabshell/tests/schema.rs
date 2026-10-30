//! Checks the JSON Schemas in `crabshell/schema/` against the code and fixtures.
//!
//! The schemas are generated from the serde types that parse `sandbox.toml`
//! and `[package.metadata.build-rs]`, so they can't drift from what crabshell
//! accepts. Run with `UPDATE_SCHEMA=1` to regenerate them after changing a
//! type. The e2e fixtures and the built-in grants are validated against them.

use std::path::{Path, PathBuf};

use crabshell::config::SandboxConfig;
use crabshell::declared::Declared;
use schemars::JsonSchema;
use schemars::generate::SchemaSettings;
use serde_json::Value;

/// The schema for `sandbox.toml`.
const SANDBOX: &str = "sandbox.schema.json";
/// The schema for a `[package.metadata.build-rs]` table.
const BUILD_RS: &str = "build-rs.schema.json";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Generate the schema for `T`, as draft-07 for the widest editor support.
fn generate<T: JsonSchema>(title: &str) -> Value {
    let mut schema = SchemaSettings::draft07().into_generator().into_root_schema_for::<T>();
    schema.insert("title".into(), title.into());
    let mut schema = serde_json::to_value(schema).unwrap();
    drop_null(&mut schema);
    schema
}

/// Remove `null` from optional keys' types: TOML has no null, optional keys
/// are simply absent.
fn drop_null(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if let Some(Value::Array(any_of)) = map.get_mut("anyOf") {
                any_of.retain(|branch| branch != &serde_json::json!({ "type": "null" }));
                if let [only] = any_of.as_slice() {
                    let only = only.clone();
                    map.remove("anyOf");
                    if let Value::Object(only) = only {
                        map.extend(only);
                    }
                }
            }
            if let Some(Value::Array(types)) = map.get_mut("type") {
                types.retain(|ty| ty != "null");
                if let [only] = types.as_slice() {
                    let only = only.clone();
                    map.insert("type".into(), only);
                }
            }
            map.values_mut().for_each(drop_null);
        }
        Value::Array(items) => items.iter_mut().for_each(drop_null),
        _ => {}
    }
}

fn schemas() -> [(&'static str, Value); 2] {
    [
        (SANDBOX, generate::<SandboxConfig>("crabshell sandbox.toml")),
        (BUILD_RS, generate::<Declared>("crabshell [package.metadata.build-rs]")),
    ]
}

fn schema(file: &str) -> Value {
    let text = std::fs::read_to_string(root().join("schema").join(file)).unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn schemas_are_up_to_date() {
    let update = std::env::var_os("UPDATE_SCHEMA").is_some();
    for (file, generated) in schemas() {
        let path = root().join("schema").join(file);
        let text = serde_json::to_string_pretty(&generated).unwrap() + "\n";
        if update {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, text).unwrap();
        } else {
            let current = std::fs::read_to_string(&path).unwrap_or_default();
            assert!(current == text, "{file} is stale: rerun with UPDATE_SCHEMA=1");
        }
    }
}

/// Fail with every schema violation in the TOML file at `path`.
fn assert_valid(schema: &Value, path: &Path, instance: &Value) {
    let validator = jsonschema::validator_for(schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|err| format!("{} at {}", err, err.instance_path()))
        .collect();
    assert!(errors.is_empty(), "{}:\n{}", path.display(), errors.join("\n"));
}

fn toml(path: &Path) -> Value {
    toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn fixtures_and_builtins_match_the_schemas() {
    let sandbox = schema(SANDBOX);
    let build_rs = schema(BUILD_RS);
    let builtins = root().join("src/grants.toml");
    assert_valid(&sandbox, &builtins, &toml(&builtins));

    let fixtures = root().join("../e2e-tests");
    let mut checked = 0;
    for fixture in std::fs::read_dir(&fixtures).unwrap() {
        let fixture = fixture.unwrap().path();
        if !fixture.is_dir() {
            continue;
        }
        for file in std::fs::read_dir(&fixture).unwrap() {
            let path = file.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if name.starts_with("sandbox.") && name.ends_with(".toml") {
                assert_valid(&sandbox, &path, &toml(&path));
                checked += 1;
            } else if name == "requests.toml" {
                // A requests file maps package names to declarations.
                let requests = toml(&path);
                for declared in requests["package"].as_object().unwrap().values() {
                    assert_valid(&build_rs, &path, declared);
                }
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "no fixtures found under {}", fixtures.display());
}

#[test]
fn the_schemas_reject_unknown_keys_and_bad_names() {
    let sandbox = schema(SANDBOX);
    let validator = jsonschema::validator_for(&sandbox).unwrap();
    let reject = |text: &str| !validator.is_valid(&toml::from_str::<Value>(text).unwrap());

    assert!(reject("[external]\nreed = []\n"));
    assert!(reject("[grant.Bad]\nrun = []\n"));
    assert!(reject("[variables]\nBad = \"/usr\"\n"));
    assert!(reject("[package.\"*\"]\nenv = []\n"));
    assert!(reject("[external]\nread = { inherit = true, valu = [\"/x\"] }\n"));
    assert!(!reject("[external]\ngrants = [\"cc\"]\nread = { inherit = true, value = [\"/x\"] }\n"));
}
