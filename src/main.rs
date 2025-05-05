use futures::stream::StreamExt;
use libp2p::swarm::SwarmEvent;
use std::error::Error;
use tokio::{io, io::AsyncBufReadExt, select};

use stockchain::{
    communication::{get_message_via_data, InteractionMessage},
    gossip::{Gossip, GossipEvent, MyBehaviourEvent}
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut gossip = Gossip::new()?;
    gossip.join_room("public_test")?;
    gossip.open_ears()?;

    // Read full lines from stdin
    let rooms: Vec<String> = gossip.topics.iter().map(|x| x.0.clone()).collect();
    println!("Self Id: {:?}\nRooms joined: {:?}", gossip.peer_id(), rooms);
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Kick it off
    loop {
        select! {
            Ok(Some(line)) = stdin.next_line() => {
                let data = parse_command(&mut gossip, line.as_str());
                let Some(data) = data else {
                    continue;
                };
                let room = gossip.fetch_room_from_name(&data.1);
                let Some(room) = room else {
                    println!("Invalid room given");
                    continue;
                };
                if let Err(e) = gossip.gossip(&data.0, room) {
                    println!("Publish error: {e:?}");
                }
            }
            event = gossip.swarm.select_next_some() => handle_event(&mut gossip, event),
        }
    }
}

fn handle_event(gossip: &mut Gossip, event: SwarmEvent<MyBehaviourEvent>) {
    let Some(action) = gossip.handle_event(event) else {
        return;
    };
    let GossipEvent::Message(data) = action else {
        println!("Event: {action:?}");
        return;
    };
    let message = match get_message_via_data(&data) {
        Ok(message) => message,
        Err(e) => {
            println!("Error parsing message: {e:?}");
            return;
        }
    };
    match message {
        InteractionMessage::Ping => {
            println!("Ping received");
        }
        InteractionMessage::SharedSecretExchange(shared_secret_exchange) => {
            println!("Shared secret exchange: {:?}", shared_secret_exchange);
        }
        InteractionMessage::SharedSecretExchangeResponse(response) => {
            println!("Shared secret exchange response: {:?}", response);
        }
        InteractionMessage::SharedSecretCommunication(communication) => {
            println!("Shared secret communication: {:?}", communication);
        }
        InteractionMessage::RequestPublicKey => {
            println!("Request public key received");
        }
        InteractionMessage::Other => {
            println!("Other message received");
        }
    }
}

fn parse_command(gossip: &mut Gossip, command: &str) -> Option<(InteractionMessage, String)> {
    let args: Vec<String> = command.split(" ").map(|s| s.to_string()).collect();
    if args.len() < 2 {
        println!("<cmd> <room> <info?>");
        return None;
    }
    let cmd= match args[0].as_str() {
        "ping" => InteractionMessage::Ping,
        "join_room" => {
            println!("{:?}", gossip.join_room(&args[1]));
            return None;
        },
        "request_public_key" => InteractionMessage::RequestPublicKey,
        "shared_secret_exchange" => todo!(),
        "shared_secret_exchange_response" => todo!(),
        "shared_secret_communication" => InteractionMessage::SharedSecretCommunication("communication".to_string()),
        _ => InteractionMessage::Other,
    };
    Some((cmd, args[1].clone()))
}