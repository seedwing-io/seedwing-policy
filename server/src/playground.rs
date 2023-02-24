use seedwing_policy_engine::lang::builder::Builder as PolicyBuilder;
use seedwing_policy_engine::runtime::sources::{Directory, Ephemeral};

#[derive(Clone)]
pub struct PlaygroundState {
    builder: PolicyBuilder,
    sources: Vec<Directory>,
}

impl PlaygroundState {
    pub fn new(builder: PolicyBuilder, sources: Vec<Directory>) -> Self {
        Self { builder, sources }
    }

    pub fn build(&self, policy: &[u8]) -> Result<PolicyBuilder, String> {
        let mut builder = self.builder.clone();
        for source in self.sources.iter() {
            if let Err(e) = builder.build(source.iter()) {
                log::error!("err {:?}", e);
                return Err(e
                    .iter()
                    .map(|b| b.to_string())
                    .collect::<Vec<String>>()
                    .join(","));
            }
        }
        match core::str::from_utf8(policy) {
            Ok(s) => {
                if let Err(e) = builder.build(Ephemeral::new("playground", s).iter()) {
                    log::error!("unable to build policy [{:?}]", e);
                    return Err(format!("Compilation error: {e:?}"));
                }
            }
            Err(e) => {
                log::error!("unable to parse [{:?}]", e);
                return Err(format!("Unable to parse POST'd input {e:?}"));
            }
        }
        Ok(builder)
    }
}
