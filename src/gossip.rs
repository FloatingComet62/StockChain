use std::{
    collections::hash_map::DefaultHasher,
    error::Error,
    fmt::Display,
    hash::{Hash, Hasher},
    time::Duration,
};

use libp2p::{
    gossipsub::{self, MessageId},
    mdns, noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use tokio::io;
use tracing_subscriber::EnvFilter;

// We create a custom network behaviour that combines Gossipsub and Mdns.
#[derive(NetworkBehaviour)]
pub struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

pub struct Gossip {
    pub swarm: libp2p::Swarm<MyBehaviour>,
    pub topics: Vec<gossipsub::IdentTopic>,
}

pub enum GossipEvent {
    NewConnection(Vec<libp2p::PeerId>),
    Disconnection(Vec<libp2p::PeerId>),
    Message(libp2p::PeerId, String),
}
impl Display for GossipEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GossipEvent::NewConnection(peers) => write!(f, "New connection: {:?}", peers),
            GossipEvent::Disconnection(peers) => write!(f, "Disconnection: {:?}", peers),
            GossipEvent::Message(peer_id, message) => {
                write!(f, "Message from {}: {}", peer_id, message)
            }
        }
    }
}

impl Gossip {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();

        let swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_quic()
            .with_behaviour(|key| {
                // To content-address message, we can take the hash of message and use it as an ID.
                let message_id_fn = |message: &gossipsub::Message| {
                    let mut s = DefaultHasher::new();
                    message.data.hash(&mut s);
                    gossipsub::MessageId::from(s.finish().to_string())
                };

                // Set a custom gossipsub configuration
                let gossipsub_config = gossipsub::ConfigBuilder::default()
                    .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
                    .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message
                    // signing)
                    .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
                    .build()
                    .map_err(io::Error::other)?; // Temporary hack because `build` does not return a proper `std::error::Error`.

                // build a gossipsub network behaviour
                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                )?;

                let mdns = mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    key.public().to_peer_id(),
                )?;
                Ok(MyBehaviour { gossipsub, mdns })
            })?
            .build();
        Ok(Self {
            swarm,
            topics: Vec::new(),
        })
    }
    pub fn join_room(&mut self, topic: &str) -> Result<(), Box<dyn Error>> {
        let topic = gossipsub::IdentTopic::new(topic);
        self.topics.push(topic.clone());

        self.swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
        Ok(())
    }
    pub fn open_ears(&mut self) -> Result<(), Box<dyn Error>> {
        // Before opening ears, we join a room with the name of our peer id, so that if someone wants to relay a message
        // specifically to us, they can do so by sending it to our peer id.
        // note that since the peer id is public, this room is not for sensitive messages.
        // encrypted messages can be used to communicate privately.
        // note: also encrypted messages can be used to establish a private room as well.
        //! CHECK BEFORE FURTHER IMPLEMENTATION: IS IT POSSIBLE TO LIST ALL THE ROOMS

        self.join_room(&self.swarm.local_peer_id().to_string())?;
        
        // Listen on all interfaces and whatever port the OS assigns
        // self.swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
        self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
        Ok(())
    }
    pub fn gossip(
        &mut self,
        message: &str,
        topic: gossipsub::IdentTopic,
    ) -> Result<MessageId, gossipsub::PublishError> {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic, message.as_bytes())
    }
    pub fn handle_event(&mut self, event: SwarmEvent<MyBehaviourEvent>) -> Option<GossipEvent> {
        match event {
            SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                let mut peers = Vec::with_capacity(list.len());
                for (peer_id, _multiaddr) in list {
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .add_explicit_peer(&peer_id);
                    peers.push(peer_id);
                }
                return Some(GossipEvent::NewConnection(peers));
            }
            SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                let mut peers = Vec::with_capacity(list.len());
                for (peer_id, _multiaddr) in list {
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .remove_explicit_peer(&peer_id);
                    peers.push(peer_id);
                }
                return Some(GossipEvent::Disconnection(peers));
            }
            SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source: peer_id,
                message_id: _,
                message,
            })) => {
                let message = String::from_utf8_lossy(&message.data);
                return Some(GossipEvent::Message(peer_id, message.to_string()));
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Local node is listening on {address}");
            }
            _ => {}
        }
        None
    }
}
