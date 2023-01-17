use seedwing_policy_engine::{
    lang::builder::Builder, runtime::sources::Ephemeral, value::RuntimeValue,
};

async fn evaluate(policy: &str, path: &str, data: RuntimeValue) {
    let src = Ephemeral::new("policy", policy);

    let mut builder = Builder::new();

    let path = format!("policy::{}", path);

    let _ = builder.build(src.iter());
    match builder.finish().await {
        Ok(runtime) => match runtime.evaluate(&path, data).await {
            Ok(result) => {
                println!("satisfied: {}", result.satisfied());
            }
            Err(e) => {
                eprintln!("error during evaluation: {:?}", e);
            }
        },
        Err(e) => {
            eprintln!("error building policy: {:?}", e);
        }
    }
}

pub fn run(policy: &str, path: &str, typevalue: &str) {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    if let Some((t, val)) = typevalue.split_once(":") {
        executor.block_on(evaluate(
            policy,
            &path,
            match t {
                "json" => serde_json::from_str::<serde_json::Value>(val)
                    .unwrap()
                    .into(),
                "string" => RuntimeValue::String(val.to_string()),
                "integer" => RuntimeValue::Integer(val.parse().unwrap()),
                "decimal" => RuntimeValue::Decimal(val.parse().unwrap()),
                "boolean" => RuntimeValue::Boolean(val.parse().unwrap()),
                _ => todo!(),
            },
        ));
    }
}
