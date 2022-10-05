use std::future::Future;
use std::pin::Pin;

pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + Sync + 'a>>;
pub type StaticBoxedFuture<T> = BoxedFuture<'static, T>;

#[derive(Debug, Clone, Copy)]
pub struct ParentActor(pub ActorID);

pub mod util;

pub mod start_child;
use agner_actors::ActorID;
pub use start_child::{InitType, StartChild, StartChildError, WithAck};

pub mod args_factory;
pub use args_factory::ArgsFactory;

mod stop_child;

pub mod produce_child;
pub use produce_child::ProduceChild;
