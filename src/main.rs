use futures::StreamExt;
use libp2p::gossipsub::{self, IdentTopic};
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};

use libp2p::{identity, Multiaddr};
use std::{error::Error, time::Duration};
use tracing_subscriber::EnvFilter;

struct Command {
    topic: IdentTopic,
    message: String,
}

async fn publish_message(gossipsub: &mut gossipsub::Behaviour, topic: IdentTopic, message: String) {
    let result = gossipsub.publish(topic, message.as_bytes());
    match result {
        Ok(_) => println!("Message published: {}", message),
        Err(e) => eprintln!("Error publishing message: {:?}", e),
    }
}

pub(crate) fn gossipsub_ident_topic(network: &str, topic: &str) -> IdentTopic {
    let network = network.trim_matches('/');
    let topic = topic.trim_matches('/');
    let s = format!("/{network}/{topic}");
    IdentTopic::new(s)
}

#[derive(NetworkBehaviour)]
struct Behaviour {
    gossipsub: gossipsub::Behaviour,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    //? 1. Basic config
    let p2p_local_keypair = identity::Keypair::generate_ed25519();

    //? 2. Gossip protocol behaviour config
    let header_sub_topic = gossipsub_ident_topic("mocha-4", "/header-sub/v0.0.1");
    let message_authenticity = gossipsub::MessageAuthenticity::Signed(p2p_local_keypair.clone());
    let config = gossipsub::ConfigBuilder::default()
        .validation_mode(gossipsub::ValidationMode::Strict)
        .validate_messages()
        .build()
        .unwrap();
    // build a gossipsub network behaviour
    let mut gossipsub: gossipsub::Behaviour =
        gossipsub::Behaviour::new(message_authenticity, config).unwrap();
    if gossipsub.subscribe(&header_sub_topic).is_ok() {
        println!("Subscribed to topic: {:?}", header_sub_topic);
    } else {
        eprintln!("Failed to subscribe to topic: {:?}", header_sub_topic);
    }
    //? 3. Swarm behaviour config
    let behaviour = Behaviour { gossipsub };
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(p2p_local_keypair)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::tls::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_behaviour(|_| behaviour)?
        .build();

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    if let Some(addr) = std::env::args().nth(1) {
        let remote: Multiaddr = addr.parse()?;
        swarm.dial(remote)?;
        println!("Dialed {addr}")
    }

    // Create a channel for sending commands to the main event loop
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Command>(32);

    // Spawn a separate task for sending publish commands
    tokio::spawn({
        let tx = tx.clone();
        async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                tx.send(Command {
                    topic: header_sub_topic.clone(),
                    message: "hello world".to_string(),
                })
                .await
                .unwrap();
            }
        }
    });

    loop {
        tokio::select! {
            Some(cmd) = rx.recv() => {
                publish_message(&mut swarm.behaviour_mut().gossipsub, cmd.topic, cmd.message).await;
            }
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => println!("Listening on {address:?}"),
                    SwarmEvent::Behaviour(BehaviourEvent::Gossipsub(gossipsub::Event::Message {
                        propagation_source,
                        message_id:_,
                        message,
                    })) => {
                        println!(
                            "Received message from {:?}: {}",
                            propagation_source,
                            String::from_utf8_lossy(&message.data)
                        );
                    },
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        println!("Connected to {:?}", peer_id);
                    },
                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        println!("Disconnected from {:?}", peer_id);
                    },
                    _ => {}
                }
            }
        }
    }
}
