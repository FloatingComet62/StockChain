use std::{
    collections::{hash_map::DefaultHasher, HashMap},
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
use oqs::sig::{Sig, Algorithm};
use rand::{rngs::ThreadRng, fill};
use tokio::io;
use tracing_subscriber::EnvFilter;

use crate::communication::InteractionMessage;


static NOUNCE_LEN: usize = 16;

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
    pub private_key: oqs::sig::SecretKey,
    pub public_key: oqs::sig::PublicKey,
    pub shared_secret: HashMap<libp2p::PeerId, oqs::kem::SharedSecret>,
    pub nounce_thread: ThreadRng,
}

#[derive(Debug)]
pub enum Room {
    PublicRoom(String),
    DirectMessage(String),
}
impl Display for Room {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Room::PublicRoom(name) => write!(f, "PublicRoom({})", name),
            Room::DirectMessage(name) => write!(f, "DirectMessage({})", name),
        }
    }
}

#[derive(Debug)]
pub struct MessageData {
    pub peer: libp2p::PeerId,
    pub message: String,
    pub room: Room,
}

#[derive(Debug)]
pub enum GossipEvent {
    NewConnection(Vec<libp2p::PeerId>),
    Disconnection(Vec<libp2p::PeerId>),
    Message(MessageData),
}
impl Display for GossipEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GossipEvent::NewConnection(peers) => write!(f, "New connection: {:?}", peers),
            GossipEvent::Disconnection(peers) => write!(f, "Disconnection: {:?}", peers),
            GossipEvent::Message(data) => {
                write!(f, "Message from {}({}): {}", data.peer, data.room, data.message)
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
            
        let sig = Sig::new(Algorithm::MlDsa87)?;
        let (public_key, private_key) = sig.keypair()?;
        Ok(Self {
            swarm,
            topics: Vec::new(),
            private_key,
            public_key,
            shared_secret: HashMap::new(),
            nounce_thread: rand::rng(),
        })
    }
    pub fn keys(&self) -> (oqs::sig::PublicKey, oqs::sig::SecretKey) {
        (self.public_key.clone(), self.private_key.clone())
    }
    pub fn join_room(&mut self, topic_str: &str) -> Result<(), Box<dyn Error>> {
        let topic = gossipsub::IdentTopic::new(topic_str);
        self.topics.push((topic_str.to_string(), topic.clone()));

        self.swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
        Ok(())
    }
    pub fn leave_room(&mut self, topic_str: &str) -> Result<(), Box<dyn Error>> {
        let topic = gossipsub::IdentTopic::new(topic_str);
        self.topics.retain(|(t, _)| t != topic_str);
        let _ = self.swarm.behaviour_mut().gossipsub.unsubscribe(&topic);
        Ok(())
    }
    pub fn open_ears(&mut self) -> Result<(), Box<dyn Error>> {
        // Before opening ears, we join a room with the name of our peer id, so that if someone wants to relay a message
        // specifically to us, they can do so by sending it to our peer id.
        // note that since the peer id is public, this room is not for sensitive messages.
        // encrypted messages can be used to communicate privately.
        // note: also encrypted messages can be used to establish a private room as well.
        //! CHECK BEFORE FURTHER IMPLEMENTATION: IS IT POSSIBLE TO LIST ALL THE ROOMS = GOOD THING I DID, YES THEY CAN

        self.join_room(&self.swarm.local_peer_id().to_string())?;
        
        // Listen on all interfaces and whatever port the OS assigns
        // self.swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
        self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
        Ok(())
    }
    pub fn gossip(
        &mut self,
        message: &InteractionMessage,
        topic: gossipsub::IdentTopic,
    ) -> Result<MessageId, GossipSendError> {
        let data = self.add_nounce(serde_json::to_string(message)?.as_bytes());
        Ok(self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic, data)?)
    }
    // Duplicate messages are apparantly not allowed, so we need to add a nounce to the message
    pub fn add_nounce(&self, message: &[u8]) -> Vec<u8> {
        let mut nounce = [0; NOUNCE_LEN];
        fill(&mut nounce);
        let mut data = Vec::with_capacity(message.len() + nounce.len());
        data.extend_from_slice(&nounce);
        data.extend_from_slice(message);
        data
    }
    pub fn remove_nounce(message: &[u8]) -> Vec<u8> {
        let mut data = Vec::with_capacity(message.len() - NOUNCE_LEN);
        data.extend_from_slice(&message[NOUNCE_LEN..]);
        data
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
                let data = Self::remove_nounce(&message.data);
                let content = String::from_utf8_lossy(&data);
                return Some(GossipEvent::Message(MessageData {
                    peer: peer_id,
                    message: content.to_string(),
                    room: self.get_topic_name_from_hash(message.topic),
                }));
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Local node is listening on {address}");
            }
            _ => {}
        }
        None
    }
    pub fn get_topic_name_from_hash(&self, topic: gossipsub::TopicHash) -> Room {
        for t in &self.topics {
            if t.1.hash() == topic {
                return self.get_room_from_topic(t.0.clone());
            }
        }
        panic!("Topic not found");
    }
    pub fn get_room_from_topic(&self, topic: String) -> Room {
        if topic == self.swarm.local_peer_id().to_string() {
            return Room::DirectMessage(topic);
        }
        Room::PublicRoom(topic)
    }
}
