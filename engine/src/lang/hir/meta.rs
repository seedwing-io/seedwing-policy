use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttributeValues {
    pub values: HashMap<String, Option<String>>,
}

impl AttributeValues {
    /// Get an iterator to flags, entries which only have a key, but no value.
    pub fn flags(&self) -> impl Iterator<Item = &str> {
        self.values
            .iter()
            .filter_map(|(key, value)| match value.is_some() {
                // we don't have a value, so it's a flag
                false => Some(key.as_str()),
                // we have a value, so its a field name
                true => None,
            })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metadata {
    pub documentation: Option<String>,
    pub attributes: HashMap<String, AttributeValues>,
}
