use async_std::{io, task};
use futures::prelude::*;
use libp2p::{
    build_development_transport, identity,
    kad::{
        record::store::MemoryStore, Kademlia, KademliaEvent, PeerRecord,
        PutRecordOk, QueryResult, Record,
    },
    mdns::{Mdns, MdnsEvent},
    swarm::NetworkBehaviourEventProcess,
    NetworkBehaviour, PeerId, Swarm,
};
use nettest::handler;
use std::{
    error::Error,
    io::Read,
    task::{Context, Poll},
};

fn main() -> Result<(), Box<dyn Error>> {
    // Create a new key for this peer's identity
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    // Setup up an encrypted, DNS-enabled TCP transport over
    // the Mplex protocol.
    // TODO: Replace this with a manual, stable, upgraded transport
    // like the one constructed in `transport.rs`
    let transport = build_development_transport(local_key)?;

    // Create a custom network behavior, combining Kademlia and mDNS
    #[derive(NetworkBehaviour)]
    struct MyBehavior {
        kademlia: Kademlia<MemoryStore>,
        mdns: Mdns, // TODO: Use bootstrapping here as well (for testing)
    }

    // Start implementing the necessary handlers for `MyBehavior`,
    // which includes handlers for both mDNS and Kademlia
    impl NetworkBehaviourEventProcess<MdnsEvent> for MyBehavior {
        // Called when `mdns` (in a MyBehavior instance) produces an event.
        fn inject_event(&mut self, event: MdnsEvent) {
            // If the event is a discovery event (that is, if the event
            // represents (a) peer(s) getting discovered, then do something with
            // the IDs of the discovered peers). In this case, "something" is
            // adding the addresses of those peers to the kademila dht (which
            // is necessary for the dht to work properly).
            if let MdnsEvent::Discovered(list_of_peers) = event {
                // for every peer in the list of the peers that were just
                // discovered, add that peer's identity information to the
                // kad dht's list of identities.
                for (peer_id, multiaddr) in list_of_peers {
                    // println!(
                    //     "mDNS: discovered peer {:?} {:?}",
                    //     &peer_id, &multiaddr
                    // );
                    self.kademlia.add_address(&peer_id, multiaddr);
                }
            }
        }
    }

    impl NetworkBehaviourEventProcess<KademliaEvent> for MyBehavior {
        // Called when `kademila` (in MyBehavior) produces an event.
        fn inject_event(&mut self, message: KademliaEvent) {
            // Kademlia DHTs have a few different "messages." A message is just
            // the type of action that is being acted on the dht, such as getting
            // a record or storing a record. Simply put, its just an event.
            match message {
                // If the event is a `QueryResult`, do something.
                // A `QueryResult` is an event representing when a query to the
                // dht has produced a result. Check out libp2p::kad::KademliaEvent
                // for all the variants. Right now, we only care about the QueryResult
                // event because that is all that this simple dht needs to support:
                // putting and retrieving records.
                KademliaEvent::QueryResult { id, result, stats } => {
                    // The result here is an enum
                    // with its own variants representing the types of query results
                    // that are possible, such as the query being a PUT or a GET.
                    // There are many things that you can do with a kad dht,
                    // and queries are simply one of those things
                    // (and there are different types of them!).
                    match result {
                        // If the query was a record being fetched (and it succeeded),
                        QueryResult::GetRecord(Ok(ok)) => {
                            // For each record that was fetched in all of the fetched
                            // records...
                            for PeerRecord {
                                record: Record { key, value, .. },
                                ..
                            } in ok.records
                            {
                                // ... do something with the record (print it, in this case)
                                println!(
                                    "kad dht: got record {:?} {:?} with id {:?} and stats {:?}\n",
                                    std::str::from_utf8(key.as_ref())
                                        .unwrap(),
                                    std::str::from_utf8(&value).unwrap(),
                                    id, stats,
                                );
                            }
                        }

                        // If the query was a record being fetched (and it failed)
                        QueryResult::GetRecord(Err(err)) => {
                            eprintln!(
                                "kad dht: failed to get record: {:?}",
                                err
                            );
                        }

                        // If the query was a record being stored (a put)
                        QueryResult::PutRecord(Ok(PutRecordOk {
                            key,
                        })) => {
                            println!(
                                "kad dht: successfully put record {:?}",
                                std::str::from_utf8(key.as_ref()).unwrap()
                            );
                        }

                        // If the query was a record being stored (and it failed)
                        QueryResult::PutRecord(Err(err)) => {
                            eprintln!(
                                "kad dht: failed to put record: {:?}",
                                err
                            );
                        }
                        _ => {} // We only care about getting and putting
                    }
                }
                _ => {} // We only need to worry about queries to this dht
            } // end big match
        } // end method
    } // end impl

    // The custom network behavior implementation is finished. Now it is time to
    // use it, which is done by building a `Swarm`.

    // Create a swarm to manage peers and events on those peers.
    // This manages the entire network as a whole.
    let mut swarm = {
        // Create a Kademlia behavior
        let store = MemoryStore::new(local_peer_id.clone());
        let kademlia = Kademlia::new(local_peer_id.clone(), store);

        // Create a mdns behavior
        let mdns = Mdns::new()?;

        // Instantiate the custom network behavior `MyBehavior`
        let behavior = MyBehavior { kademlia, mdns };

        // Create a new swarm with the transport, behavior, and local peer identity
        Swarm::new(transport, behavior, local_peer_id)
    };

    // Read full lines from stdin
    // let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Listen on all interfaces and whatever port the OS assigns

    Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse()?)?;
    if let Some(address) = Swarm::listeners(&swarm).next() {
        println!("listening on {:?}", address);
    }

    // Start handling lines from stdin
    loop {
        let mut line = String::new();
        std::io::stdin()
            .read_to_string(&mut line)
            .expect("could not read from stdin");
        eprintln!("I am going to handle a line right now");
        handler::handle_input_line(&mut swarm.kademlia, line);
        eprintln!("just handled a line");
    }

    /*
    task::block_on(future::poll_fn(move |cx: &mut Context<'_>| {
        // This loop exists to continuously read from stdin
        loop {
            // Try to poll the next line from the stdin stream
            match stdin.try_poll_next_unpin(cx)? {
                // If stdin received a full line, handle it.
                Poll::Ready(Some(line)) => {
                    handler::handle_input_line(&mut swarm.kademlia, line)
                }

                // If stdin broke
                Poll::Ready(None) => panic!("stdin closed"),

                // If there is no line that was entered, break out of the loop
                Poll::Pending => break,
            }
        }

        Poll::Pending
    }))
    */
}
