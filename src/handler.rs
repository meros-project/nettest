use libp2p::{
    identity,
    kad::{
        record::{store::MemoryStore, Key},
        Kademlia, Quorum, Record,
    },
};

use std::fs::File;
use std::io::prelude::*;

pub fn handle_input_line(
    kademlia: &mut Kademlia<MemoryStore>,
    line: String,
) {
    File::create("output.log")
        .unwrap()
        .write_all(b"handling a line!")
        .unwrap();

    let mut args = line.split(" ");
    match args.next() {
        Some("GET") => {
            let key = match args.next() {
                Some(key) => Key::new(&key),
                None => {
                    eprintln!("expected a key");
                    return;
                }
            };

            kademlia.get_record(&key, Quorum::One);
        }
        Some("PUT") => {
            let key = match args.next() {
                Some(key) => Key::new(&key),
                None => {
                    eprintln!("Expected a key");
                    return;
                }
            };

            let value = match args.next() {
                Some(value) => value.as_bytes().to_vec(),
                None => {
                    eprintln!("Expected value");
                    return;
                }
            };

            let record = Record {
                key,
                value,
                publisher: None,
                expires: None,
            };

            kademlia
                .put_record(record, Quorum::One)
                .expect("Failed to store record locally");
        }
        _ => {
            eprintln!("Expected GET or PUT");
        }
    }
}
