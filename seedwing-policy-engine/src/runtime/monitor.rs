use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver};
use tokio::sync::mpsc::Sender;
use futures_util::Stream;
use tokio_stream::wrappers::ReceiverStream;
use crate::runtime::TypeName;
use crate::value::RuntimeValue;

#[derive(Clone, Debug)]
pub enum MonitorResult {
    Satisified,
    Unsatisfied,
    Error,
}

pub struct Monitor {
    registrations: Vec<Registration>,
}

#[derive(Clone, Debug)]
pub struct Registration {
    name: String,
    sender: Sender<Entry>,
    stale: bool,
}

impl Registration {}

#[derive(Clone, Debug)]
pub struct Entry {
    name: TypeName,
    input: Arc<RuntimeValue>,
    result: MonitorResult,
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            registrations: vec![]
        }
    }

    pub async fn record<R: Into<MonitorResult>>(&mut self, name: TypeName, input: Arc<RuntimeValue>, result: R) {
        let entry = Entry {
            name: name.clone(),
            input,
            result: result.into(),
        };

        println!("accept entry {:?}", entry);
        println!("accepted to regs now --> {:?}", self.registrations);

        for reg in &mut self.registrations.iter_mut() {
            println!("{} vs {}", name, reg.name);
            if name.as_type_str() == reg.name {
                if reg.sender.send(entry.clone()).await.is_err() {
                    reg.stale = true
                }
            }
        }

        let mut stale_purged = Vec::default();

        for reg in &self.registrations {
            if reg.stale {
                println!("remove registration {:?}", reg);
                // skip
            } else {
                stale_purged.push(reg.clone())
            }
        }

        self.registrations = stale_purged
    }

    pub fn monitor(&mut self, name: String) -> impl Stream<Item=Entry> {
        let (sender, receiver) = channel(3);
        let reg = Registration {
            name,
            sender,
            stale: false,
        };

        println!("added registration {:?}", reg);

        self.registrations.push(reg);

        println!("regs now --> {:?}", self.registrations);

        ReceiverStream::new(receiver)
    }
}