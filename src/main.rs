use futures::stream::StreamExt;
use libp2p::swarm::SwarmEvent;
use std::error::Error;
use tokio::{io, io::AsyncBufReadExt, select};

use stockchain::{
    communication::{InteractionMessage, SharedSecretExchange, SharedSecretExchangeResponse},
    gossip::{Gossip, GossipEvent, MyBehaviourEvent, room::GossipRooms},
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
                let room = gossip.get_topic_from_name(&data.1);
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
    let GossipEvent::Message((data, message)) = action else {
        println!("Event: {action:?}");
        return;
    };
    match message {
        InteractionMessage::Ping => println!("Ping received"),
        InteractionMessage::SharedSecretExchange(shared_secret_exchange) => {
            println!("Shared secret exchange");
            let Ok(response) = gossip.secret.receive_shared_secret(
                data.peer,
                shared_secret_exchange.kem_pk,
                shared_secret_exchange.signature,
                shared_secret_exchange.pk,
            ) else {
                println!("Error receiving shared secret");
                return;
            };
            data.reply_to_peer(
                gossip,
                &InteractionMessage::SharedSecretExchangeResponse(
                    SharedSecretExchangeResponse::new(response.0, response.1, response.2),
                ),
            );
        }
        InteractionMessage::SharedSecretExchangeResponse(response) => {
            println!("Shared secret exchange response");
            let Err(e) = gossip.secret.receive_shared_secret_response(
                data.peer,
                response.kem_ct,
                response.signature,
                response.pk,
            ) else {
                return;
            };
            println!("Error receiving shared secret response {e:?}");
        }
        InteractionMessage::SharedSecretCommunication(communication) => {
            println!("Shared secret communication");
            let Ok(data) = gossip
                .secret
                .decrypt(data.peer, communication.0, communication.1)
            else {
                println!("Error decrypting data");
                return;
            };
            println!("Decrypted data: {:?}", String::from_utf8(data));
        }
        InteractionMessage::RequestPublicKey => {
            println!("Request public key received");
            data.reply_to_room(
                gossip,
                &InteractionMessage::ReplyPublicKey(gossip.secret.public_key.clone()),
            );
        }
        InteractionMessage::ReplyPublicKey(public_key) => {
            println!("Reply public key received: {:?}", public_key)
        }
        InteractionMessage::Other(e) => println!("Other message received: {:?}", e),
    }
}

fn parse_command(gossip: &mut Gossip, command: &str) -> Option<(InteractionMessage, String)> {
    let args: Vec<String> = command.split(" ").map(|s| s.to_string()).collect();
    if args.len() < 2 {
        println!("<cmd> <room> <info?>");
        return None;
    }
    let cmd = match args[0].as_str() {
        "ping" | "p" => InteractionMessage::Ping,
        "join_room" | "jr" => {
            println!("{:?}", gossip.join_room(&args[1]));
            return None;
        }
        "request_public_key" | "rpk" => InteractionMessage::RequestPublicKey,
        "shared_secret_exchange" | "sse" => {
            let Some(peer_id) = gossip.get_peer_from_room_name(&args[1]) else {
                println!("Invalid peer id");
                return None;
            };
            let Ok((kem_pk, signature, pk)) = gossip.secret.send_shared_secret(*peer_id) else {
                println!("Error sending shared secret");
                return None;
            };
            InteractionMessage::SharedSecretExchange(SharedSecretExchange::new(
                kem_pk, signature, pk,
            ))
        }
        "shared_secret_communication" | "ssc" => {
            let Some(peer_id) = gossip.get_peer_from_room_name(&args[1]) else {
                println!("Invalid peer id");
                return None;
            };
            let Ok(data) = gossip.secret.encrypt(*peer_id, get_msg(&args).as_bytes()) else {
                println!("Error encrypting data");
                return None;
            };
            InteractionMessage::SharedSecretCommunication(data)
        }
        _ => InteractionMessage::Other("fuck".to_string()),
    };
    Some((cmd, args[1].clone()))
}

fn get_msg(args: &Vec<String>) -> String {
    if args.len() < 3 {
        return "BLANK_MSG".to_string();
    }
    args[2..].join(" ")
}
