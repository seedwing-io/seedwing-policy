use yew_nested_router::Target;

mod index;
mod playground;
mod policy;
mod statistics;
mod monitor;

pub use index::*;
pub use playground::*;
pub use policy::*;
pub use statistics::*;
pub use monitor::*;

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
