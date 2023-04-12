use gloo_storage::Storage;

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ExampleData {
    pub definition: String,
    pub value: String,
    pub policy: String,
}

const DEFAULT_POLICY: &str = r#"pattern dog = {
    name: string,
    trained: boolean
}"#;

const DEFAULT_VALUE: &str = r#"name: goodboy
trained: true"#;

const KEY_DEFAULT: &str = "playground.defaultExample";

impl Default for ExampleData {
    fn default() -> Self {
        Self {
            definition: DEFAULT_POLICY.to_string(),
            value: DEFAULT_VALUE.to_string(),
            policy: "dog".to_string(),
        }
    }
}

impl ExampleData {
    pub fn load_default() -> Self {
        gloo_storage::LocalStorage::get(KEY_DEFAULT).unwrap_or_default()
    }

    pub fn store_default(example: ExampleData) {
        let _ = gloo_storage::LocalStorage::set(KEY_DEFAULT, example);
    }

    pub fn clear_default() {
        gloo_storage::LocalStorage::delete(KEY_DEFAULT);
    }
}
