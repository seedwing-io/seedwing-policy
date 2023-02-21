use yew_nested_router::Target;

mod index;
mod playground;
mod policy;

pub use index::*;
pub use playground::*;
pub use policy::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Target)]
pub enum AppRoute {
    #[default]
    #[target(index)]
    Index,
    Policy {
        path: String,
    },
    Playground,
}
