use yew_nested_router::Target;

mod documentation;
mod index;
mod playground;
mod repository;

pub use documentation::*;
pub use index::*;
pub use playground::*;
pub use repository::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Target)]
pub enum AppRoute {
    #[default]
    #[target(index)]
    Index,
    Repository {
        path: String,
    },
    Documentation,
    Examples,
    Playground,
}
