use libp2p::{
    PeerId, gossipsub, mdns, noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use std::{
    collections::{HashSet, hash_map::DefaultHasher},
    error::Error,
    fmt::Display,
    hash::{Hash, Hasher},
    time::Duration,
};
use tokio::io;
use tracing_subscriber::EnvFilter;

use crate::communication::InteractionMessage;

pub mod events;
pub mod impls;
pub mod message;
pub mod nonce;
pub mod room;
pub mod secret;

use events::EventHandler;
use message::MessageData;
use nonce::Nonce;
use room::GossipRooms;
use secret::Secret;

#[derive(NetworkBehaviour)]
pub struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

#[derive(Debug)]
pub enum GossipSendError {
    PublishError(gossipsub::PublishError),
    SerdeError(serde_json::Error),
}
impl From<gossipsub::PublishError> for GossipSendError {
    fn from(err: gossipsub::PublishError) -> Self {
        GossipSendError::PublishError(err)
    }
}
impl From<serde_json::Error> for GossipSendError {
    fn from(err: serde_json::Error) -> Self {
        GossipSendError::SerdeError(err)
    }
}

pub struct Gossip {
    pub swarm: libp2p::Swarm<MyBehaviour>,
    pub topics: Vec<(String, gossipsub::IdentTopic)>,
    pub peer_ids: HashSet<PeerId>,
    pub secret: Secret,
    pub nonce: Nonce,
}

#[derive(Debug)]
pub enum GossipEvent {
    NewConnection(Vec<libp2p::PeerId>),
    Disconnection(Vec<libp2p::PeerId>),
    Message((MessageData, InteractionMessage)),
}
impl Display for GossipEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GossipEvent::NewConnection(peers) => write!(f, "New connection: {:?}", peers),
            GossipEvent::Disconnection(peers) => write!(f, "Disconnection: {:?}", peers),
            GossipEvent::Message(data) => {
                write!(
                    f,
                    "Message from {}({}): {} | {:?}",
                    data.0.peer, data.0.room, data.0.message, data.1
                )
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
            peer_ids: HashSet::new(),
            secret: Secret::new()?,
            nonce: Nonce::new(),
        })
    }
    pub fn peer_id(&self) -> PeerId {
        self.swarm.local_peer_id().clone()
    }
    pub fn open_ears(&mut self) -> Result<(), Box<dyn Error>> {
        // Before opening ears, we join a room with the name of our peer id, so that if someone wants to relay a message
        // specifically to us, they can do so by sending it to our peer id.
        // note that since the peer id is public, this room is not for sensitive messages.
        // encrypted messages can be used to communicate privately.
        // note: also encrypted messages can be used to establish a private room as well.
        //! CHECK BEFORE FURTHER IMPLEMENTATION: IS IT POSSIBLE TO LIST ALL THE ROOMS = GOOD THING I DID, YES THEY CAN

        let last_five_id_char = self.peer_id().generate_room_name();
        self.join_room(&last_five_id_char)?;

        // Listen on all interfaces and whatever port the OS assigns
        // self.swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
        self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
        Ok(())
    }
    pub fn gossip(
        &mut self,
        message: &InteractionMessage,
        topic: gossipsub::IdentTopic,
    ) -> Result<gossipsub::MessageId, GossipSendError> {
        let data = self
            .nonce
            .add_nonce(serde_json::to_string(message)?.as_bytes());
        Ok(self.swarm.behaviour_mut().gossipsub.publish(topic, data)?)
    }
    pub fn handle_event(&mut self, event: SwarmEvent<MyBehaviourEvent>) -> Option<GossipEvent> {
        EventHandler::handle(self, event)
    }
}

pub trait GenerateRoomName {
    fn generate_room_name(&self) -> String;
}

impl GenerateRoomName for PeerId {
    fn generate_room_name(&self) -> String {
        // let mut hasher = DefaultHasher::new();
        // peer_id.hash(&mut hasher);
        // let hash = hasher.finish();
        // format!("public_{hash}")
        let s = self.to_string();
        let n = s.char_indices().nth_back(4).unwrap().0;
        s[n..].to_string()
    }
}
