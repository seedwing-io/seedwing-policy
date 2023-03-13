use crate::package::Package;
use crate::runtime::PackagePath;

#[cfg(feature = "sigstore")]
mod envelope;

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["intoto"]));
    pkg.register_source("".into(), include_str!("envelope.dog"));
    #[cfg(feature = "sigstore")]
    pkg.register_function("verify-envelope".into(), envelope::Verify);
    pkg
}

#[cfg(test)]
mod test {
    use crate::lang::builder::Builder;
    use crate::runtime::sources::Ephemeral;
    use crate::runtime::EvalContext;
    use serde_json::json;

    #[actix_rt::test]
    async fn envelope() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern envelope = intoto::envelope
        "#,
        );

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let input = json!({
        "payloadType": "application/vnd.in-toto+json",
        "payload": "eyJfdHlwZSI6Imh0dHBzOi8vaW4tdG90by5pby9TdGF0ZW1lbnQvdjAuMSIsInByZWRpY2F0ZVR5cGUiOiJodHRwczovL3Nsc2EuZGV2L3Byb3ZlbmFuY2UvdjAuMiIsInN1YmplY3QiOm51bGwsInByZWRpY2F0ZSI6eyJidWlsZGVyIjp7ImlkIjoiaHR0cHM6Ly90ZWt0b24uZGV2L2NoYWlucy92MiJ9LCJidWlsZFR5cGUiOiJ0ZWt0b24uZGV2L3YxYmV0YTEvVGFza1J1biIsImludm9jYXRpb24iOnsiY29uZmlnU291cmNlIjp7fSwicGFyYW1ldGVycyI6e319LCJidWlsZENvbmZpZyI6eyJzdGVwcyI6W3siZW50cnlQb2ludCI6IiMhL3Vzci9iaW4vZW52IHNoXG5lY2hvICdnY3IuaW8vZm9vL2JhcicgfCB0ZWUgL3Rla3Rvbi9yZXN1bHRzL1RFU1RfVVJMXG5lY2hvICdzaGEyNTY6MDVmOTViMjZlZDEwNjY4YjcxODNjMWUyZGE5ODYxMGU5MTM3MmZhOWY1MTAwNDZkNGNlNTgxMmFkZGFkODZiNScgfCB0ZWUgL3Rla3Rvbi9yZXN1bHRzL1RFU1RfRElHRVNUIiwiYXJndW1lbnRzIjpudWxsLCJlbnZpcm9ubWVudCI6eyJjb250YWluZXIiOiJjcmVhdGUtaW1hZ2UiLCJpbWFnZSI6ImRvY2tlci5pby9saWJyYXJ5L2J1c3lib3hAc2hhMjU2OmMxMThmNTM4MzY1MzY5MjA3YzEyZTU3OTRjM2NiZmI3YjA0MmQ5NTBhZjU5MGFlNmMyODdlZGU3NGYyOWI3ZDQifSwiYW5ub3RhdGlvbnMiOm51bGx9XX0sIm1ldGFkYXRhIjp7ImJ1aWxkU3RhcnRlZE9uIjoiMjAyMy0wMy0xMlQwOTo0MDoxNloiLCJidWlsZEZpbmlzaGVkT24iOiIyMDIzLTAzLTEyVDA5OjQwOjIxWiIsImNvbXBsZXRlbmVzcyI6eyJwYXJhbWV0ZXJzIjpmYWxzZSwiZW52aXJvbm1lbnQiOmZhbHNlLCJtYXRlcmlhbHMiOmZhbHNlfSwicmVwcm9kdWNpYmxlIjpmYWxzZX19fQ==",
        "signatures": [
          {
            "keyid": "SHA256:caEJWYJSxy1SVF2KObm5Rr3Yt6xIb4T2w56FHtCg8WI",
            "sig": "MEQCICuvg0XqwCECEySkoHmsTJ+ktW9ISzGXsp3GQDaBSam6AiAj/g+3duDtEI9ud4aF/Fb4w9y5og7UNrmO5t9TxUfVrw=="
          }
        ]
        });
        let result = runtime
            .evaluate("test::envelope", input, EvalContext::default())
            .await;
        //println!("result: {:?}", result);
        assert!(result.as_ref().unwrap().satisfied());

        //let _output = result.unwrap().output().unwrap();
    }
}
