use criterion::{criterion_group, criterion_main, Criterion};
use seedwing_policy_engine::lang::lir::EvalContext;
use seedwing_policy_engine::{lang::builder::Builder, runtime::sources::Ephemeral};
use serde_json::json;
use serde_json::Value as JsonValue;

fn eval_speed(bencher: &mut Criterion, data: TestData) {
    let mut builder = Builder::new();

    let _ = builder.build(data.src.iter());

    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let runtime = executor.block_on(builder.finish()).unwrap();

    bencher.bench_function(&format!("{} eval", data.id), |b| {
        b.iter(|| {
            executor
                .block_on(runtime.evaluate(&data.path, &data.value, EvalContext::default()))
                .unwrap();
        })
    });
}

fn build_speed(bencher: &mut Criterion, data: TestData) {
    bencher.bench_function(&format!("{} build", data.id), |b| {
        b.iter(|| {
            let mut builder = Builder::new();
            let _ = builder.build(data.src.clone().iter());
        })
    });
}

fn end_to_end_speed(bencher: &mut Criterion, data: TestData) {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    bencher.bench_function(&format!("{} end 2 end", data.id), |b| {
        b.iter(|| {
            let data = data.clone();
            executor.block_on(async move {
                let mut builder = Builder::new();

                let _ = builder.build(data.src.iter());
                let runtime = builder.finish().await.unwrap();
                runtime
                    .evaluate(&data.path, &data.value, EvalContext::default())
                    .await
                    .unwrap()
            });
        })
    });
}

#[derive(Clone)]
struct TestData {
    id: &'static str,
    src: Ephemeral,
    path: String,
    value: JsonValue,
}

/*
 * Smoke test group
 *
 * Basic tests with small in-memory examples.
 */
fn smoke_eval(bencher: &mut Criterion) {
    eval_speed(bencher, testdata_smoke());
}

fn smoke_build(bencher: &mut Criterion) {
    build_speed(bencher, testdata_smoke());
}

fn smoke_end_to_end(bencher: &mut Criterion) {
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
        id: "smoke",
        src,
        path: "smoke::bob".to_string(),
        value: json!(
            {
                "name": "Bob"
            }
        ),
    }
}

/*
 * Sigstore test group
 *
 * More complex tests using sigstore verification.
 */

//fn sigstore_eval(bencher: &mut Criterion) {
//    eval_speed(bencher, testdata_sigstore());
//}
//
//fn sigstore_end_to_end(bencher: &mut Criterion) {
//    end_to_end_speed(bencher, testdata_sigstore());
//}

fn sigstore_build(bencher: &mut Criterion) {
    build_speed(bencher, testdata_sigstore());
}

fn testdata_sigstore() -> TestData {
    let src = Ephemeral::new(
        "foo::bar",
        r#"
            // Single-line comment, yay
            pattern signed-thing = {
                digest: sigstore::sha256(
                    n<1>::{
                        apiVersion: "0.0.1",
                        spec: {
                            signature: {
                                publicKey: {
                                    content: base64::base64(
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
    );

    TestData {
        id: "sigstore",
        src,
        path: "foo::bar::signed-thing".to_string(),
        value,
    }
}

criterion_group!(
    benches,
    smoke_build,
    smoke_eval,
    smoke_end_to_end,
    sigstore_build,
    // sigstore_eval,
    // sigstore_end_to_end
);
criterion_main!(benches);
