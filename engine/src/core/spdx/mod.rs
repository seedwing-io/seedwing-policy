use crate::package::Package;
use crate::runtime::PackagePath;

mod compatible;
mod license;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["spdx"]));
    pkg.register_source("license".into(), include_str!("license.dog"));
    pkg.register_source("v2_2".into(), include_str!("spdx-v2.2.dog"));
    pkg.register_source("v2_3".into(), include_str!("spdx-v2.3.dog"));
    pkg.register_function("compatible".into(), compatible::Compatible);
    pkg.register_function("license-expr".into(), license::Expression);
    pkg
}

#[cfg(test)]
mod test {
    use crate::{
        assert_satisfied, runtime::testutil::test_data_dir, runtime::testutil::test_pattern,
    };
    use std::fs;

    #[tokio::test]
    async fn spdx_2_3_predicate() {
        let input =
            fs::read_to_string(test_data_dir().join("spdx").join("v2_3_predicate.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&input).unwrap();
        let result = test_pattern(r#"spdx::v2_3::predicate"#, json).await;
        assert_satisfied!(&result);
    }

    #[tokio::test]
    async fn spdx_2_2_predicate() {
        let input =
            fs::read_to_string(test_data_dir().join("spdx").join("v2_2_predicate.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&input).unwrap();
        let result = test_pattern(r#"spdx::v2_2::predicate"#, json).await;
        assert_satisfied!(&result);
    }

    #[tokio::test]
    async fn spdx_2_2_statement() {
        let input =
            fs::read_to_string(test_data_dir().join("spdx").join("v2_2_statement.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&input).unwrap();
        let result = test_pattern(r#"spdx::v2_2::statement"#, json).await;
        assert_satisfied!(&result);
    }
}
