use async_std::{io, task};
use futures::prelude::*;
use libp2p::{
    build_development_transport, identity,
    kad::{
        record::{store::MemoryStore, Key},
        Kademlia, KademliaEvent, PeerRecord, PutRecordOk, QueryResult,
        Quorum, Record,
    },
    mdns::{Mdns, MdnsEvent},
    swarm::NetworkBehaviourEventProcess,
    NetworkBehaviour, PeerId, Swarm,
};
use std::{
    error::Error,
    task::{Context, Poll},
};

fn main() -> Result<(), Box<dyn Error>> {
    // Create a new key for this peer's identity
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PerrId::from(local_key.public());

    // Setup up an encrypted, DNS-enabled TCP transport over
    // the Mplex protocol.
    // TODO: Replace this with a manual, stable, upgraded transport
    let transport = build_development_transport(local_key)?;

    // Create a custom network behavior, combining Kademila and mDNS
    #[derive(NetworkBehaviour)]
    struct MyBehavior {
        kademila: Kademila<MemoryStore>,
        mdns: Mdns, // TODO: Use bootstrapping here as well (for testing)
    }

    // Start implementing the necessary handlers for `MyBehavior`,
    // which includes handlers for both mDNS and Kademila
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
                    println!(
                        "mDNS: discovered peer {:?} {:?}",
                        &peer_id, &multiaddr
                    );
                    self.kademila.add_address(&peer_id, multiaddr);
                }
            }
        }
    }

    impl NetworkBehaviourEventProcess<KademilaEvent> for MyBehavior {
        // Called when `kademila` (in MyBehavior) produces an event.
        fn inject_event(&mut self, message: KademilaEvent) {
            // Kademila DHTs have a few different "messages." A message is just
            // the type of action that is being acted on the dht, such as getting
            // a record or storing a record. Simply put, its just an event.
            match message {
                // If the event is a `QueryResult`, do something.
                // A `QueryResult` is an event representing when a query to the
                // dht has produced a result. Check out libp2p::kad::KademilaEvent
                // for all the variants. Right now, we only care about the QueryResult
                // event because that is all that this simple dht needs to support:
                // putting and retrieving records.
                KademilaEvent::QueryResult { result, .. } => {
                    // The result here is an enum
                    // with its own variants representing the types of query results
                    // that are possible, such as the query being a PUT or a GET.

                    // Match the result,
                    match result {
                        // If the query was a record being fetched (and it succeeded),
                        // do something.
                        QueryResult::GetRecord(Ok(ok)) => {
                            for PeerRecord {
                                record: Record { key, value, .. },
                                ..
                            } in ok.records
                            {
                                println!(
                                    "kad dht: Got record {:?} {:?}",
                                    std::str::from_utf8(key.as_ref())
                                        .unwrap(),
                                    std::str::from_utf8(&value).unwrap()
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
