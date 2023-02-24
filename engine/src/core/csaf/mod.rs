use crate::package::Package;
use crate::runtime::{PackagePath, World};

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["csaf"]));
    pkg.register_source("".into(), include_str!("v2_0.dog"));
    pkg
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::core::test::test_pattern;
    use serde_json::json;

    #[tokio::test]
    async fn test_csaf_valid() {
        let input = include_str!("rhba-2023_0564.json");
        let json: serde_json::Value = serde_json::from_str(input).unwrap();
        let result = test_pattern(r#"csaf::csaf"#, json).await;

        assert!(result.satisfied());
    }
}
