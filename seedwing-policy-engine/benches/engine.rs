use bencher::{benchmark_group, benchmark_main, Bencher};
use futures::executor::block_on;
use seedwing_policy_engine::{lang::builder::Builder, runtime::sources::Ephemeral};
use serde_json::json;

fn eval_speed(bencher: &mut Bencher) {
    let data = testdata1();
    let mut builder = Builder::new();

    let _ = builder.build(data.src.iter());
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
    let _ = builder.build(data.src.iter());

    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    bencher.iter(|| {
        let _ = executor.block_on(builder.finish()).unwrap();
    });
}

fn end_to_end_speed(bencher: &mut Bencher) {
    let data = testdata1();
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    bencher.iter(|| {
        let data = data.clone();
        executor.block_on(async move {
            let mut builder = Builder::new();

            let _ = builder.build(data.src.iter());
            let runtime = builder.finish().await.unwrap();
            runtime
                .evaluate(&data.path, data.value.clone())
                .await
                .unwrap()
        });
    });
}

#[derive(Clone)]
struct TestData {
    src: Ephemeral,
    path: String,
    value: String,
}

fn testdata1() -> TestData {
    let src = Ephemeral::new(
        "test1",
        r#"
        pattern named<name> = {
            name: name
        }

        pattern jim = named<"Jim">
        pattern bob = named<"Bob">

        pattern folks = jim || bob

        "#,
    );

    TestData {
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

benchmark_group!(
    benches,
    build_speed,
    finish_speed,
    eval_speed,
    end_to_end_speed
);
benchmark_main!(benches);
