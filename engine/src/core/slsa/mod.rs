use crate::package::Package;
use crate::runtime::PackagePath;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["slsa"]))
        .with_documentation("Packages related to [SLSA](https://slsa.dev) documents.");
    pkg.register_source("v1_0".into(), include_str!("provenance-v1_0.dog"));
    pkg.register_source("github".into(), include_str!("provenance-github-v1_0.dog"));
    pkg.register_source("v0_2".into(), include_str!("provenance-v0_2.dog"));
    pkg
}

#[cfg(test)]
mod tests {
    use crate::{assert_satisfied, runtime::testutil::test_pattern};

    #[tokio::test]
    async fn test_valid_provenance_1_0() {
        let input = include_str!("example1.json");
        let json: serde_json::Value = serde_json::from_str(input).unwrap();
        let result = test_pattern(r#"slsa::v1_0::provenance"#, json).await;

        assert_satisfied!(result);
    }

    #[tokio::test]
    async fn test_github_provenance_1_0() {
        let input = include_str!("example2.json");
        let json: serde_json::Value = serde_json::from_str(input).unwrap();
        let result = test_pattern(r#"slsa::github::provenance"#, json).await;

        assert_satisfied!(result);
    }

    #[tokio::test]
    async fn test_valid_provenance_0_2() {
        let input = include_str!("example3.json");
        let json: serde_json::Value = serde_json::from_str(input).unwrap();
        let result = test_pattern(r#"slsa::v0_2::provenance"#, json).await;

        assert_satisfied!(result);
    }
}
