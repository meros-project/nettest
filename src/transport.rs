use libp2p::{
    core::upgrade, identity::Keypair, secio::SecioConfig, tcp::TcpConfig,
    yamux, Multiaddr, Transport,
};

fn get_transport_example() {
    // Before doing anything, create a new keypair for this node's
    // identity
    let keypair = Keypair::generate_ed25519();

    // Create a default tcp transport
    let base_tcp = TcpConfig::new();

    // Upgrade the transport to use secio, for authentication, and
    // yamux, for multiplexing. But first, create the default configurations
    // for these protocols.
    let secio_conf = SecioConfig::new(keypair); // Default secio config
    let yamux_conf = yamux::Config::default(); // Default yamux config

    // Actually upgrade the transport from the base tcp layer
    let transport = base_tcp
        .upgrade(upgrade::Version::V1) // Upgrade the network
        .authenticate(secio_conf) // Negotiate secio as the authentication protocol
        .multiplex(yamux_conf); // Negotiate yamux as the multiplexing protocol

    // Parse a multiaddr of the peer to connect to
    let addr: Multiaddr = "/ip4/192.168.1.208/tcp/20000"
        .parse()
        .expect("invalid multiaddr");

    // Dial the peer (this returns a future)
    let _conn = transport.dial(addr).unwrap();
}
