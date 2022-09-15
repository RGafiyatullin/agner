use agner::actors::{ArcError, System};
use agner::sup::fixed::ChildSpec;

mod room {
    use agner::actors::{ActorID, BoxError, Context, Event, ExitReason};
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::Arc;

    use super::conn;

    pub enum Message {
        Join(ActorID, SocketAddr),
        Post(ActorID, Arc<str>),

        ConnDown(ActorID, Arc<ExitReason>),
    }

    pub async fn run(context: &mut Context<Message>, _arg: ()) -> Result<(), BoxError> {
        context.init_ack(Default::default());

        let mut participants = HashMap::new();

        loop {
            match context.next_event().await {
                Event::Signal { .. } => unreachable!(),

                Event::Message(Message::ConnDown(actor_id, exit_reason)) => {
                    if let Some(addr) = participants.remove(&actor_id) {
                        for participant_actor_id in participants.keys().copied() {
                            context
                                .system()
                                .send(
                                    participant_actor_id,
                                    conn::Message::Left(addr, Arc::clone(&exit_reason)),
                                )
                                .await;
                        }
                    }
                },
                Event::Message(Message::Join(actor_id, peer_addr)) => {
                    for participant_actor_id in participants.keys().copied() {
                        context
                            .system()
                            .send(participant_actor_id, conn::Message::Joined(peer_addr))
                            .await;
                    }

                    participants.insert(actor_id, peer_addr);

                    let system = context.system();
                    let notification = async move {
                        let conn_down = system.wait(actor_id);
                        let exit_reason = conn_down.await;
                        Message::ConnDown(actor_id, exit_reason)
                    };
                    context.future_to_inbox(notification).await;
                },
                Event::Message(Message::Post(actor_id, message)) => {
                    if let Some(from_addr) = participants.get(&actor_id).copied() {
                        for participand_actor_id in
                            participants.keys().copied().filter(|p| *p != actor_id)
                        {
                            context
                                .system()
                                .send(
                                    participand_actor_id,
                                    conn::Message::Posted(from_addr, Arc::clone(&message)),
                                )
                                .await;
                        }
                    }
                },
            }
        }
    }
}

mod conn {
    use agner::actors::{BoxError, Context, Event, ExitReason};
    use agner::sup::Registered;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpStream;

    use super::room;

    pub struct Args {
        pub tcp_stream: TcpStream,
        pub peer_addr: SocketAddr,
        pub room: Registered,
    }

    pub enum Message {
        Joined(SocketAddr),
        Left(SocketAddr, Arc<ExitReason>),
        Posted(SocketAddr, Arc<str>),
    }

    pub async fn run(context: &mut Context<Message>, mut args: Args) -> Result<(), BoxError> {
        context.init_ack(Default::default());

        context
            .system()
            .send(
                args.room.get().ok_or("room is not ready")?,
                room::Message::Join(context.actor_id(), args.peer_addr),
            )
            .await;

        let (read_half, mut write_half) = args.tcp_stream.split();
        let mut read_lines = BufReader::new(read_half).lines();

        loop {
            tokio::select! {
                next_line = read_lines.next_line() => {
                    let next_line = next_line?;
                    let next_line = next_line.ok_or("EOF")?;

                    context.system().send(args.room.get().ok_or("room is not ready")?, room::Message::Post(context.actor_id(), next_line.into())).await;
                },
                event = context.next_event() => {
                    match event {
                        Event::Message(Message::Joined(addr)) => {
                            let message = format!("JOINED [{}]\n", addr);
                            write_half.write_all(message.as_bytes()).await?;
                            write_half.flush().await?;
                        }
                        Event::Message(Message::Left(addr, reason)) => {
                            let message = format!("LEFT [{}]: {}\n", addr, reason.pp());
                            write_half.write_all(message.as_bytes()).await?;
                            write_half.flush().await?;
                        }
                        Event::Message(Message::Posted(from, message)) => {
                            let message = format!("[{}] {}\n", from, message);
                            write_half.write_all(message.as_bytes()).await?;
                            write_half.flush().await?;
                        },
                        Event::Signal {..} => unreachable!()
                    }
                }
            }
        }
    }
}

mod acceptor {
    use std::net::SocketAddr;

    use agner::actors::{BoxError, Context};
    use agner::sup::{dynamic, Registered};
    use tokio::net::TcpListener;

    #[derive(Debug, Clone)]
    pub struct Args {
        pub conn_sup: Registered,
        pub bind_addr: SocketAddr,
    }

    pub type Message = std::convert::Infallible;

    pub async fn run(context: &mut Context<Message>, args: Args) -> Result<(), BoxError> {
        let tcp_listener = TcpListener::bind(args.bind_addr).await?;
        context.init_ack(Default::default());

        loop {
            let (tcp_stream, peer_addr) = tcp_listener.accept().await?;
            dynamic::start_child(
                &context.system(),
                args.conn_sup.get().ok_or("conn-sup is not ready")?,
                (tcp_stream, peer_addr),
            )
            .await?;
        }
    }
}

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    let _ = pretty_env_logger::try_init_timed();

    run().await.expect("Failure")
}

async fn run() -> Result<(), ArcError> {
    let system = System::new(Default::default());

    let restart_strategy = ();

    let top_sup_spec = {
        use agner::sup::{dynamic, fixed};

        let room = agner::sup::Registered::new();
        let conn_sup = agner::sup::Registered::new();
        let acceptor = agner::sup::Registered::new();

        fixed::SupSpec::new(restart_strategy)
            .with_child(
                fixed::child_spec(room::run, fixed::arg_clone(())).register(room.to_owned()),
            )
            .with_child(
                fixed::child_spec(
                    dynamic::dynamic_sup,
                    fixed::arg_call({
                        let room = room.to_owned();
                        move || {
                            dynamic::child_spec(conn::run, {
                                let room = room.to_owned();
                                move |(tcp_stream, peer_addr)| conn::Args {
                                    room: room.to_owned(),
                                    tcp_stream,
                                    peer_addr,
                                }
                            })
                        }
                    }),
                )
                .register(conn_sup.to_owned()),
            )
            .with_child(
                fixed::child_spec(
                    acceptor::run,
                    fixed::arg_clone(acceptor::Args {
                        bind_addr: "127.0.0.1:8090".parse().unwrap(),
                        conn_sup: conn_sup.to_owned(),
                    }),
                )
                .register(acceptor),
            )
    };

    let top_sup = system
        .spawn(agner::sup::fixed::fixed_sup, top_sup_spec, Default::default())
        .await?;

    Err(system.wait(top_sup).await)
}
