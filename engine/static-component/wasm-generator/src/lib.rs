use anyhow::Result;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{Read, Seek, Write};
use std::path::PathBuf;
use std::process::Command;
use toml_edit::Document;
use wasm_compose::composer::ComponentComposer;
use wasm_compose::config::Config as ComposerConfig;
use wasm_compose::config::{Instantiation, InstantiationArg};
use wit_component::ComponentEncoder;

macro_rules! workspace_toml {
    ($policy_name:expr) => {
        format!(
            "
[workspace]
members = [
    \"{}\",
]
[workspace.dependencies]
wit-bindgen = {{ version = \"0.9.0\", default-features = true, features = ['macros'] }}
",
            $policy_name
        )
    };
}

macro_rules! workspace_member_toml {
    ($policy_name:expr) => {
        format!(
            "
[package]
name = \"{}\"
version = \"0.1.0\"
edition = \"2021\"

[dependencies]
wit-bindgen = {{ workspace = true }}

[lib]
crate-type = [\"cdylib\"]
",
            $policy_name
        )
    };
}

const SRC_HEADER: &str = r#"
wit_bindgen::generate!({
    inline: "
      package seedwing:policy

      world static-world {
        export static-config
      }

      interface static-types {
        record config {
          policy: string,
          policy-name: string,
        }
      }

      interface static-config {
        use static-types.{config}
        policy-config: func() -> config
      }
    ",
    macro_export,
});
"#;

macro_rules! config_lib {
    ($policy:expr, $policy_name:expr) => {
        format!(
            "{}
use crate::exports::seedwing::policy::static_config::Config;
use crate::exports::seedwing::policy::static_config::StaticConfig;

struct PolicyConfig;

impl StaticConfig for PolicyConfig {{
    fn policy_config() -> Config {{
        let policy = r#\"{}\"#;
        let policy_name = r#\"{}\"#;
        Config {{
            policy: policy.to_string(),
            policy_name: policy_name.to_string(),
        }}
    }}
}}
export_static_world!(PolicyConfig);
    ",
            SRC_HEADER, $policy, $policy_name
        )
    };
}

pub enum BuildType {
    Debug,
    Release,
}

impl BuildType {
    fn as_string(&self) -> &'static str {
        match self {
            BuildType::Debug => "debug",
            BuildType::Release => "release",
        }
    }
}

pub struct GenConfig {
    pub policy: String,
    pub policy_name: String,
    pub build_type: BuildType,
    pub modules_dir: PathBuf,
    pub output_dir: PathBuf,
}

impl GenConfig {
    pub fn new(policy: &str, policy_name: &str) -> Self {
        Self {
            policy: policy.to_string(),
            policy_name: policy_name.to_string(),
            build_type: BuildType::Debug,
            modules_dir: PathBuf::from("modules"),
            output_dir: PathBuf::from("target"),
        }
    }
}

fn create_workspace(working_dir_path: &PathBuf, policy_name: &str) -> Result<()> {
    let workspace_toml = format!("{}/Cargo.toml", working_dir_path.display());
    if !working_dir_path.exists() {
        fs::create_dir_all(working_dir_path)?;
        fs::write(workspace_toml, workspace_toml!(policy_name))?;
    }
    Ok(())
}

fn add_workspace_member(working_dir_path: &PathBuf, policy_name: &str) -> Result<()> {
    let workspace_toml = format!("{}/Cargo.toml", working_dir_path.display());
    let mut cargo_toml = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&workspace_toml)?;
    let mut content = String::new();
    cargo_toml.read_to_string(&mut content)?;

    let mut root_toml = content.parse::<Document>()?;
    let members = root_toml["workspace"]["members"].as_array_mut().unwrap();
    if members
        .iter()
        .find(|i| i.as_str() == Some(policy_name))
        .is_some()
    {
        return Ok(());
    }
    members.push(policy_name);

    cargo_toml.set_len(0)?;
    cargo_toml.seek(std::io::SeekFrom::Start(0))?;
    cargo_toml.write_all(root_toml.to_string().as_bytes())?;
    Ok(())
}

pub fn generate(config: &GenConfig) -> Result<PathBuf> {
    let policy_name = &config.policy_name.clone();
    let working_dir_path = PathBuf::from("working");
    create_workspace(&working_dir_path, policy_name)?;
    add_workspace_member(&working_dir_path, policy_name)?;

    let working_dir = working_dir_path.display();
    let policy_src_dir = format!("{}/{}/src", working_dir, &policy_name);
    let policy_toml = format!("{}/{}/Cargo.toml", working_dir, &policy_name);
    let _res = fs::create_dir_all(&policy_src_dir);
    let _res = fs::write(&policy_toml, workspace_member_toml!(&policy_name));
    let _res = fs::write(
        format!("{}/lib.rs", &policy_src_dir),
        config_lib!(&config.policy, &config.policy_name),
    );
    let output = match config.build_type {
        BuildType::Release => Command::new("cargo")
            // TODO: Currently when doing a release build there is an issue with wit.
            .args(&[
                "build",
                "--package",
                policy_name,
                "--release",
                "--manifest-path",
                &policy_toml,
                "--target",
                "wasm32-unknown-unknown",
            ])
            .output()?,
        BuildType::Debug => Command::new("cargo")
            .args(&[
                "build",
                "--package",
                policy_name,
                "--manifest-path",
                &policy_toml,
                "--target",
                "wasm32-unknown-unknown",
            ])
            .output()?,
    };
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Compilation error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    // Make a component out for the core webassembly module that we compiled
    // above.
    let config_module = fs::read(format!(
        "{}/target/wasm32-unknown-unknown/{}/{}.wasm",
        working_dir,
        config.build_type.as_string(),
        policy_name,
    ))
    .unwrap();
    let component_encoder = ComponentEncoder::default()
        .module(config_module.as_slice())
        .unwrap();
    let component = component_encoder.encode().unwrap();
    let component_path = PathBuf::from(format!(
        "{}/target/{}-static-config-component.wasm",
        working_dir, policy_name
    ));
    println!("Writing component to {component_path:?}");
    let ret = fs::write(&component_path, component);
    if ret.is_err() {
        return Err(anyhow::anyhow!(
            "Failed to write component to {component_path:?}",
            component_path = component_path
        ));
    }
    println!("Created webassembly component: {component_path:?}");

    // Next compose component with another component
    let mut inst_args = indexmap::IndexMap::new();
    inst_args.insert(
        "seedwing:policy/engine".to_string(),
        InstantiationArg {
            instance: "seedwing-policy-engine-component.wasm".to_string(),
            export: None,
        },
    );
    inst_args.insert(
        "seedwing:policy/static-config".to_string(),
        InstantiationArg {
            instance: format!(
                "{}/target/{}-static-config-component.wasm",
                working_dir, policy_name
            ),
            export: None,
        },
    );
    let instantiation = Instantiation {
        arguments: inst_args,
        ..Default::default()
    };
    let mut instantiations = indexmap::IndexMap::new();
    instantiations.insert("$input".to_string(), instantiation);

    let composer_config = ComposerConfig {
        search_paths: vec![
            config.modules_dir.clone(),
            PathBuf::from(format!("{}/target", working_dir)),
        ],
        instantiations,
        ..Default::default()
    };
    let mut static_eval_component_path = config.modules_dir.clone();
    static_eval_component_path.push("static-evaluator-component.wasm");
    let composer = ComponentComposer::new(&static_eval_component_path, &composer_config);
    let composed = composer.compose();

    let mut composed_path = config.output_dir.clone();
    if !composed_path.is_absolute() {
        composed_path = env::current_dir()?.join(composed_path)
    }

    composed_path.push(format!("{}-composed.wasm", config.policy_name));
    let res = fs::write(&composed_path, composed.unwrap());
    if res.is_err() {
        return Err(anyhow::anyhow!(
            "Failed to write composed component: {}",
            composed_path.to_string_lossy()
        ));
    }
    Ok(composed_path.to_path_buf())
}
