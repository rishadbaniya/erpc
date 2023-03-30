mod onion_perf_circuits;

// TODO : Check for Bridge Rlelay Partitioning from other(non bridge relay)
use arti_client::config::circ::PathConfigBuilder;
use arti_client::TorClient;
use arti_client::TorClientConfig;
use onion_perf_circuits::OnionPerfData;
use std::fs;
use std::sync::Arc;
use std::vec;
use tokio::sync::Mutex;
use tor_chanmgr::ChannelUsage;
use tor_circmgr::path::TorPath;
use tor_circmgr::CircMgr;
use tor_netdir::MdReceiver;
use tor_netdir::PartialNetDir;
use tor_netdir::Relay;
use tor_netdoc::doc::netstatus::Consensus;
use tor_netdoc::doc::netstatus::MdConsensus;
use tor_proto::circuit::CircParameters;
use tor_proto::circuit::ClientCirc;
use tor_proto::stream::StreamParameters;
use tor_rtcompat::Runtime;

//use onion_perf_circuits;

pub fn generate_all_relay_two_hop_circuit_combinations(
    relays: Vec<Relay>,
) -> Vec<Vec<(Relay, Relay)>> {
    let mut total_two_hops: Vec<Vec<(Relay, Relay)>> = vec![];
    for i in 1..relays.len() {
        let mut two_hops: Vec<(Relay, Relay)> = vec![];
        for j in 1..relays.len() {
            two_hops.push((relays[i].clone(), relays[j].clone()));
        }
        total_two_hops.push(two_hops);
    }
    return total_two_hops;
}

fn gen_thread() {
    std::thread::spawn(|| {
        println!("IM RIGHT HERE");
    });
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    //let mut onion_perf_data = OnionPerfData::new().await.unwrap();
    //onion_perf_data
    //.create_all_relay_to_relay_combinations()
    //.await;

    let tor_client_config = TorClientConfig::default();
    let client = TorClient::create_bootstrapped(tor_client_config)
        .await
        .unwrap();

    let net_dir = client.dirmgr().timely_netdir().unwrap();
    let relays: Vec<Relay> = net_dir.relays().into_iter().collect();

    //let end_node = {
    //for ref relay in relays{
    //if relay.is_flagged_guard()
    //}
    //};

    for i in 0..100 {
        let path = tor_circmgr::path::TorPath::new_multihop::<()>(vec![
            relays[i].clone(),
            relays[i + 1].clone(),
            relays[i + 2].clone(),
            relays[i + 3].clone(),
            relays[i + 4].clone(),
        ]);

        let circ_parameters = CircParameters::default();
        let circ_usage = ChannelUsage::UselessCircuit;

        let cir_mgr = client.circmgr();

        let t = std::time::Instant::now();
        let cir = cir_mgr
            .builder()
            .build(&path, &circ_parameters, circ_usage)
            .await;
        match cir {
            Ok(c) => {
                //println!("{:#?}", c.channel());
                println!("Time {:?}", std::time::Instant::now().duration_since(t));

                let x = c
                    .begin_stream("142.251.46.174", 80, Some(StreamParameters::default()))
                    .await;
                println!("{:#?}", x);
                c.terminate();
            }
            Err(x) => {
                //println!("{:#?}", x);
            }
        }
    }
    //    //match cir {
    //Ok(a) => {
    //println!("{:?}", a);
    ////a.terminate();
    ////println!("{:?}", a.is_closing());
    ////println!("{:#?}", a);
    //}
    //Err(b) => {}
}

//let total_two_hops_relay_circuits =
//generate_all_relay_two_hop_circuit_combinations(relays.clone());

//let x: Vec<(Relay, Relay)> = total_two_hops_relay_circuits
//.into_iter()
//.flatten()
//.collect();

//println!("the length is {:?}", x.len());

//let mut ok = Arc::new(Mutex::new(0));
//let mut err = Arc::new(Mutex::new(0));

//let cir_mgr = tor_client.circmgr().clone();

//for relay in relays {
////tokio::spawn(async move {

////});

////let path = tor_circmgr::path::TorPath::new_one_hop((relay).clone());
//let path = tor_circmgr::path::TorPath::new_multihop([relays[0], relays[1]]);
//let circ_parameters = CircParameters::default();
//let circ_usage = ChannelUsage::UselessCircuit;

//let cir = cir_mgr
//.builder()
//.build(&path, &circ_parameters, circ_usage)
//.await;
//match cir {
//Ok(a) => {
//println!("{:?}", a);
////a.terminate();
////println!("{:?}", a.is_closing());
////println!("{:#?}", a);
//}
//Err(b) => {}
//}
//[>_ok = *_ok + 1;
//} else {
//println!("I got error :{:?}", err);
//let mut _err = ok.lock().await;
//[>_err = *_err + 1;
//}
//
//.build(&path, &circ_parameters, circ_usage)
//.await;

//for relay in relays {
////let path = tor_circmgr::path::TorPath::new_multihop(vec![relays[0].clone(), relays[1].clone()]);
//let path = tor_circmgr::path::TorPath::new_one_hop(relay.clone());
//let circ_parameters = CircParameters::default();
//let circ_usage = ChannelUsage::UselessCircuit;

//if x.is_ok() {
//ok += 1;
//println!("Ok {:?}", ok);
//} else {
//err += 1;
//println!("Err {:?}", err);
//}
//tokio::task::spawn(async {});
//}

//let path = tor_circmgr::path::TorPath::new_multihop(vec![relays[0].clone(), relays[1].clone()]);

//let circ = arti_client
//.circmgr()
//.builder()
//.build(&path, &params, usage)
//.await;
//let x = Ci
//let params = CircParameters::default();
//let usage = ChannelUsage::UselessCircuit;
//let x = tor_client.di
//let net_dir = tor_client

//let x = tor_netdoc::doc::netstatus::Consensus<Relay>;
// NOTE : These are directly created from the tor_netdoc, gotta use tor_netdir to create
// abstractions on the ones of tor_netdoc
//let file_contents = fs::read_to_string("test_md_consensus").unwrap();
//let (_, _, x) = MdConsensus::parse(&file_contents).unwrap();
//let (m, t) = x.dangerously_into_parts();

//let md_consensus = m.consensus;

//let net_dir = PartialNetDir::new(md_consensus, None);

//print!("{:#?}", net_dir);
////let x = net_dir.missing_microdescs();

//println!("{:?}", net_dir.n_missing());

//TorPath::new_multihop(path);

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

//.await
//.unwrap();
