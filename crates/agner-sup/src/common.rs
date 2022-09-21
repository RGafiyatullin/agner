use std::future::Future;
use std::pin::Pin;

pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + Sync + 'a>>;
pub type StaticBoxedFuture<T> = BoxedFuture<'static, T>;

mod util;

mod start_child;
pub use start_child::{new as new_start_child, InitType, StartChild, StartChildError};

pub mod args_factory;
pub use args_factory::ArgsFactory;

mod stop_child;

mod produce_child;
pub use produce_child::{new as new_produce_child, ProduceChild};
