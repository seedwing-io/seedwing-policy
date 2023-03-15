use seedwing_policy_engine::lang::builder::Builder;
use seedwing_policy_engine::runtime::sources::Ephemeral;
use std::fs;

macro_rules! test {
    ($name:ident) => {
        #[tokio::test]
        async fn $name() {
            let source = Ephemeral::new(
                stringify!($name),
                fs::read_to_string(format!("tests/parse-data/{}.dog", stringify!($name))).unwrap(),
            );
            let mut builder = Builder::new();
            builder.build(source.iter()).unwrap();
            builder.finish().await.unwrap();
        }
    };
}

test!(and);
test!(empty);
test!(contains_self);
test!(attributes1);
