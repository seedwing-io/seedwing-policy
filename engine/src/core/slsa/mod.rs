use crate::package::Package;
use crate::runtime::PackagePath;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["slsa"]));
    pkg.register_source("provenance".into(), include_str!("provenance-v1_0.dog"));
    pkg.register_source("github".into(), include_str!("provenance-github-v1_0.dog"));
    pkg
}

#[cfg(test)]
mod tests {
    use crate::{assert_satisfied, runtime::testutil::test_pattern};

    #[tokio::test]
    async fn test_valid_provenance() {
        let input = include_str!("example1.json");
        let json: serde_json::Value = serde_json::from_str(input).unwrap();
        let result = test_pattern(r#"slsa::provenance::provenance"#, json).await;

        assert_satisfied!(result);
    }

    #[tokio::test]
    async fn test_github_provenance() {
        let input = include_str!("example2.json");
        let json: serde_json::Value = serde_json::from_str(input).unwrap();
        let result = test_pattern(r#"slsa::github::provenance"#, json).await;

        assert_satisfied!(result);
    }
}
