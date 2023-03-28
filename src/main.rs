// TODO : Check for Bridge Rlelay Partitioning from other(non bridge relay)
use arti_client::config::circ::PathConfigBuilder;
use arti_client::TorClient;
use arti_client::TorClientConfig;
use std::fs;
use tor_circmgr::path::TorPath;
use tor_circmgr::CircMgr;
use tor_netdoc::doc::netstatus::Consensus;
use tor_netdoc::doc::netstatus::MdConsensus;

#[tokio::main]
async fn main() {
    //let x = tor_netdoc::doc::netstatus::Consensus<Relay>;
    let file_contents = fs::read_to_string("xyz").unwrap();
    let (_, _, x) = MdConsensus::parse(&file_contents).unwrap();
    let (m, t) = x.dangerously_into_parts();

    println!("{:#?}", m);

    // Generate the Microdescriptor consensus document at the default storage provided by storage
    // config
    // TODO: Make use of it to explore the Relay

    //let mut tor_client_config_builder = TorClientConfig::builder();

    //let x = TorPath::new_multihop(relays);
    //let y = tor_client_config_builder.path_rules();
    //let x: AddrPortPattern = "198.98.61.11:9001".parse().unwrap();
    // Keeping the empty reachable
    //y.set_reachable_addrs(vec![]);
    //y.set_reachable_addrs(vec![x]);

    //let tor_client_config = tor_client_config_builder.build().unwrap();

    //let tor_client = TorClient::create_bootstrapped(tor_client_config)
    //.await
    //.unwrap();

    //let x = tor_client.connect(("google.com", 80)).await.unwrap();

    //println!("{:#?}", x);
}
