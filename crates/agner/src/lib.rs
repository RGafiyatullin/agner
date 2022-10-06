//! agner — an [actor](https://en.wikipedia.org/wiki/Actor_model) toolkit inspired by Erlang/OTP.
//!
//! Note: Right now this is a research project, i.e. it is possible that the API will undergo
//! incompatible changes within the version 0.3.x.
//!
//! As it has been stated, agner is inspired by Erlang/OTP, so all similarities to the actual
//! frameworks, supported or obsolete, are purely intentional. :)
//!
//!
//! # [Actors](crate::actors)
//!
//! An actor is an activity with the following properties:
//! - runs in parallel (implemented as a [`Future`](std::future::Future));
//! - has a handle ([`ActorID`](crate::actors::ActorID));
//! - can receive messages;
//! - when terminates — yields an exit reason ([`Exit`](crate::actors::Exit));
//! - any two actors can be [linked](crate::actors::Context::link) with each other:
//!     - if one of the linked actors exits with a reason other than
//!       [`Exit::normal()`](crate::actors::Exit::normal()) — the other receives an exit-signal;
//!     - if the process receiving an exit-signal does not ["trap
//!       exits"](crate::actors::Context::trap_exit), it will also be terminated.
//!
//! ## Implementing an Actor
//! The actor's behaviour is defined by:
//! - the type of its argument;
//! - the type of the message it accepts;
//! - the behaviour function.
//!
//! In order to implement an actor one should define an async function that
//! - returns a value for which the trait [`Into<Exit>`](crate::actors::Exit) is
//! defined
//! - and accepts two arguments:
//!     - a mutable reference to [`Context<Message>`](crate::actors::Context);
//!     - `Argument`.
//!
//! Example:
//! ```
//! use agner::actors::{Context, Exit, Never};
//!
//! async fn shutdown_after_six_messages(context: &mut Context<String>, actor_name: String) {
//!     for i in 0..6 {
//!         let message = context.next_message().await;
//!         eprintln!("Actor {:?} received {:?}", actor_name, message);
//!     }
//! }
//! ```
//!
//! ## Spawning an Actor
//!
//! Actors cannot run on their own, they need an [actor system](crate::actors::System) to be spawned
//! in. This is necessary to avoid having a global state imposed by mere usage of the library.
//! A [`System`](crate::actors::System) is a scope within which the actors run.
//!
//! Example:
//! ```
//! use agner::actors::{System, Context, Exit, Never};
//!
//! async fn a_dummy(context: &mut Context<Option<String>>, actor_name: String) {
//!     eprintln!("{}: hi!", actor_name);
//!
//!     while let Some(message) = context.next_message().await {
//!         eprintln!("{}: received {:?}", actor_name, message);
//!     }
//!
//!     eprintln!("{}: bye!", actor_name);
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     // create a system with default configuration
//!     let system = System::new(Default::default());
//!
//!     let actor_id = system.spawn(
//!         a_dummy,
//!         "the-dummy".to_owned(),
//!         Default::default())
//!             .await.expect("Failed to spawn an actor");
//!
//!     system.send(actor_id, Some("one".to_owned())).await;
//!     system.send(actor_id, Some("two".to_owned())).await;
//!     system.send(actor_id, Some("three".to_owned())).await;
//!     system.send(actor_id, Option::<String>::None).await;
//!
//!     let exit_reason = system.wait(actor_id).await;
//!     eprintln!("{} exited: {:?}", actor_id, exit_reason);
//! }
//! ```
//!
//! ## Terminating an Actor
//!
//! ### "Willful" Termination
//!
//! #### Returning from the Behaviour Function
//!
//! If the actor's behaviour function returns — the actor terminates.
//! The return type of the behaviour function must implement the trait
//! [`Into<Exit>`](crate::actors::Exit).
//!
//! Example:
//! ```
//! use std::convert::Infallible;
//! use agner::actors::{Context, Exit};
//!
//! async fn unit_is_normal_exit(_context: &mut Context<Infallible>, _args:()) {}
//!
//! async fn result_into_exit(_context: &mut Context<Infallible>, _args:()) -> Result<(), Exit> {
//!     Ok(()) // Equivalent to `Err(Exit::normal())`
//! }
//! ```
//!
//! #### Invoking `Context::exit`
//!
//! Example:
//! ```
//! use std::convert::Infallible;
//! use agner::actors::{Context, Exit};
//!
//! async fn normal_exit(context: &mut Context<Infallible>, args: ()) -> Infallible {
//!     context.exit(Exit::normal()).await;
//!     unreachable!()
//! }
//! ```
//!
//! ### Terminating from Outside
//!
//! An actor can be terminated by invoking [`System::exit(&self, ActorID,
//! Exit)`](crate::actors::System::exit).
//!
//! In this case an actor receives an exit-signal. If the actor ["traps
//! exits"](crate::actors::Context::trap_exit), it can perform a graceful shutdown (or even keep
//! running). That is if the exit reason is not [`Exit::kill()`](crate::actors::Exit::kill): in this
//! case the actor just terminates, and its linked actors in their turn receive exit-signals.
//!
//!
//!
//! # Supervision
//!
//! A supervisor in essence is a special actor that is responsible for starting, stopping, and
//! monitoring of other actors.
//!
//! ## `ChildFactory<Arg>`
//!
//! A child is started using a special recipe —
//! [values for which the trait `ChildFactory<Arg>` is defined](crate::sup::common::ChildFactory):
//! such recipes produce a child actor when provided an argument of type `Arg`.
//!
//! The child can fail during the attempt to start it — this is considered a *start up failure*.
//!
//! The child may crash after it has successfully started — this situation is called *runtime
//! failure*.
//!
//!
//!
//!
//! ## `uniform` — a Supervisor for Homogenous Children
//!
//! [Uniform Supervisor](crate::sup::uniform::run) is a simpler of two types of supervisors: it
//! starts children on similar nature on demand, some time after the supervisor is started.
//!
//! Although this type of supervisor does not restart its children upon their failure, it keeps
//! track on them and, upon own shutdown, will terminate all its children.
//!
//! The behaviour of the uniform supervisor is defined by its [supervisor
//! specification](crate::sup::uniform::SupSpec).
//!
//! ~~The main property of that supervision specification is a "recipe" to produce a new child —
//! [`ChildFactory<Arg>`](crate::sup::common::ChildFactory).~~
//!
//! Example:
//! ```
//! #[derive(Debug, Clone)]
//! struct Environment;
//!
//! #[derive(Debug)]
//! struct InboundConnection;
//!
//! fn start_a_uniform_sup_for_inbound_connections() -> ActorID {
//!     let produce_child =
//! }
//! ```
//!
//!
//!
//! ## `mixed` — a Supervisor for Heterogenous Children
//!
//! [Such supervisor](crate::sup::mixed::run) is a special actor that is responsible for starting,
//! stopping and monitoring its children.
//!
//! The behaviour of mixed supervisor is defined by a [supervisor
//! specification](crate::sup::mixed::SupSpec).
//!
//! The supervisor specification consists of the list of [child
//! specifications](crate::sup::mixed::ChildSpec) and a [restart
//! strategy](crate::sup::mixed::RestartStrategy).
//!
//! The children are started in the order specified in the supervior specification.
//! Upon shutdown they are stopped in the reverse order.
//!
//! The restart strategy defines the behaviour of the supervisor when a child failure occurs.
//!
//! ###

pub mod utils {
    pub use agner_utils::*;
}

pub mod actors {
    pub use agner_actors::*;
}

#[cfg(feature = "init-ack")]
pub mod init_ack {
    pub use agner_init_ack::*;
}

#[cfg(feature = "reg")]
pub mod reg {
    pub use agner_reg::*;
}

#[cfg(feature = "sup")]
pub mod sup {
    pub use agner_sup::*;
}

#[cfg(feature = "helm")]
pub mod helm {
    pub use agner_helm::*;
}

#[cfg(feature = "test-actor")]
pub mod test_actor {
    pub use agner_test_actor::*;
}
