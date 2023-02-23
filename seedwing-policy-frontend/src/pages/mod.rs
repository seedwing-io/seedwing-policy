use yew_nested_router::Target;

mod index;
mod monitor;
mod playground;
mod policy;
mod statistics;

pub use index::*;
pub use monitor::*;
pub use playground::*;
pub use policy::*;
pub use statistics::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Target)]
pub enum AppRoute {
    #[default]
    #[target(index)]
    Index,
    Policy {
        path: String,
    },
    Statistics {
        path: String,
    },
    Monitor {
        path: String,
    },
    Playground,
}
