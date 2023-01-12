use bencher::{benchmark_group, benchmark_main, Bencher};
use futures::executor::block_on;
use seedwing_policy_engine::{lang::builder::Builder, runtime::sources::Ephemeral};
use serde_json::json;

fn eval_speed(bencher: &mut Bencher) {
    let data = testdata1();
    let mut builder = Builder::new();

    let result = builder.build(data.src.iter());
    let runtime = block_on(builder.finish()).unwrap();

    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    bencher.iter(|| {
        executor
            .block_on(runtime.evaluate(&data.path, data.value.clone()))
            .unwrap()
    });
}

fn build_speed(bencher: &mut Bencher) {
    let data = testdata1();

    let mut builder = Builder::new();

    bencher.iter(|| {
        let _ = builder.build(data.src.clone().iter());
    });
}

fn finish_speed(bencher: &mut Bencher) {
    let data = testdata1();

    let mut builder = Builder::new();
    let result = builder.build(data.src.iter());

    bencher.iter(|| {
        let runtime = block_on(builder.finish()).unwrap();
    });
}

#[derive(Clone)]
struct TestData {
    id: String,
    src: Ephemeral,
    path: String,
    value: String,
}

fn testdata1() -> TestData {
    let src = Ephemeral::new(
        "test1",
        r#"
        type named<name> = {
            name: name
        }

        type jim = named<"Jim">
        type bob = named<"Bob">

        type folks = jim || bob

        "#,
    );

    TestData {
        id: "test1".to_string(),
        src,
        path: "test1::folks".to_string(),
        value: json!(
            {
                "name": "Bob",
                "age": 52,
            }
        )
        .to_string(),
    }
}

benchmark_group!(benches, build_speed, finish_speed, eval_speed);
benchmark_main!(benches);
