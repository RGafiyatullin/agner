mod behaviour;
mod child_spec;
mod sup_spec;
// mod restart_strategy;

type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send + Sync + 'static>>;

pub mod hlist;

use std::future::Future;
use std::pin::Pin;

pub use behaviour::fixed_sup;
pub use child_spec::{arg_arc, arg_call, arg_clone, child_spec, ArgFactory, ChildSpec};
pub use sup_spec::SupSpec;
// pub use restart_strategy::RestartStrategy;
