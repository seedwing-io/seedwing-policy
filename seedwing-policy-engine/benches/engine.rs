use bencher::{benchmark_group, benchmark_main, Bencher};
use seedwing_policy_engine::{lang::builder::Builder, runtime::sources::Ephemeral};
use serde_json::json;

fn eval_speed(bencher: &mut Bencher, data: TestData) {
    let mut builder = Builder::new();

    let _ = builder.build(data.src.iter());

    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let runtime = executor.block_on(builder.finish()).unwrap();

    bencher.iter(|| {
        executor
            .block_on(runtime.evaluate(&data.path, data.value.clone()))
            .unwrap()
    });
}

fn build_speed(bencher: &mut Bencher, data: TestData) {
    bencher.iter(|| {
        let mut builder = Builder::new();
        let _ = builder.build(data.src.clone().iter());
    });
}

fn end_to_end_speed(bencher: &mut Bencher, data: TestData) {
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

/*
 * Smoke test group
 *
 * Basic tests with small in-memory examples.
 */
fn smoke_eval(bencher: &mut Bencher) {
    eval_speed(bencher, testdata_smoke());
}

fn smoke_build(bencher: &mut Bencher) {
    build_speed(bencher, testdata_smoke());
}

fn smoke_end_to_end(bencher: &mut Bencher) {
    end_to_end_speed(bencher, testdata_smoke());
}

fn testdata_smoke() -> TestData {
    let src = Ephemeral::new(
        "smoke",
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
        path: "smoke::folks".to_string(),
        value: json!(
            {
                "name": "Bob",
                "age": 52,
            }
        )
        .to_string(),
    }
}

/*
 * Sigstore test group
 *
 * More complex tests using sigstore verification.
 */

fn sigstore_eval(bencher: &mut Bencher) {
    eval_speed(bencher, testdata_sigstore());
}

fn sigstore_build(bencher: &mut Bencher) {
    build_speed(bencher, testdata_sigstore());
}

fn sigstore_end_to_end(bencher: &mut Bencher) {
    end_to_end_speed(bencher, testdata_sigstore());
}

fn testdata_sigstore() -> TestData {
    let src = Ephemeral::new(
        "foo::bar",
        r#"
            // Single-line comment, yay
            pattern signed-thing = {
                digest: sigstore::SHA256(
                    n<1>::{
                        apiVersion: "0.0.1",
                        spec: {
                            signature: {
                                publicKey: {
                                    content: base64::Base64(
                                        x509::PEM( n<1>::{
                                            version: 2,
                                            extensions: n<1>::{
                                                subjectAlternativeName: n<1>::{
                                                    rfc822: "bob@mcwhirter.org",
                                                }
                                            }
                                        } )
                                    )
                                }
                            }
                        }
                    }
                )
            }
        "#,
    );

    let value = json!(
        {
            "digest": "5dd1e2b50b89874fd086da4b61176167ae9e4b434945325326690c8f604d0408"
        }
    )
    .to_string();

    TestData {
        src,
        path: "foo::bar::signed-thing".to_string(),
        value,
    }
}

benchmark_group!(
    benches,
    smoke_build,
    smoke_eval,
    smoke_end_to_end,
    sigstore_build,
    sigstore_eval,
    sigstore_end_to_end
);
benchmark_main!(benches);
