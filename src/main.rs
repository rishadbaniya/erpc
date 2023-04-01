mod onion_perf_circuits;

use arti_client::config::CfgPath;
// TODO : Check for Bridge Rlelay Partitioning from other(non bridge relay)
use arti_client::config::circ::PathConfigBuilder;
use arti_client::TorClient;
use arti_client::TorClientConfig;
use onion_perf_circuits::OnionPerfData;
use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::vec;
use tokio::net::TcpStream;
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

// NOTE : Not used rigth now
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

fn gen_thread(i: i32) -> std::thread::JoinHandle<()> {
    let handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async move {
            let mut tor_client_config = TorClientConfig::builder();
            let cfg_path = CfgPath::new(i.to_string());
            let storage_config = tor_client_config.storage();
            storage_config.cache_dir(cfg_path.clone());
            storage_config.state_dir(cfg_path.clone());

            let client = TorClient::create_bootstrapped(tor_client_config.build().unwrap())
                .await
                .unwrap();

            // Builds net_dir based on latest md_consensus
            let net_dir = client.dirmgr().timely_netdir().unwrap();
            let relays: Vec<Relay> = net_dir.relays().into_iter().map(|x| x).collect();
            let cir_mgr = client.circmgr();

            let relay_1 = relays[i as usize].clone();
            for j in 0..500 {
                let x = i as usize + j;
                let relay_2 = relays[x].clone();
                // Create two hop
                let path = tor_circmgr::path::TorPath::new_multihop::<()>(vec![
                    relay_1.clone(),
                    relay_2.clone(),
                ]);

                let circ_parameters = CircParameters::default();
                let circ_usage = ChannelUsage::UselessCircuit;

                let cir = cir_mgr
                    .builder()
                    .build(&path, &circ_parameters, circ_usage)
                    .await;
                match cir {
                    Ok(c) => {
                        println!("SUCCESS between {i} and {x}");
                        //                            //println!("{:#?}", c.channel());
                        //                            println!("Time {:?}", std::time::Instant::now().duration_since(t));
                        //
                        //                           let x = c
                        //                               .begin_stream("142.251.46.174", 80, Some(StreamParameters::default()))
                        //                               .await;
                        //                           println!("{:#?}", x);
                        //                           c.terminate();
                    }
                    Err(_) => {
                        println!("Failure {i} and {x}");
                    }
                }
            }
            println!("DID IT");
        });
    });
    handle
}

// NOTE : I've used 100 clients here because as of right now i was unable to figure out a way to do
// it in single thread through tokio::spawn (life time issues persisted, which wouldn't let me pass
// CircMgr across tasks)
fn main() {
    // let rt = tokio::runtime::Builder::new_current_thread()
    //     .build()
    //     .unwrap();

    // rt.block_on(async {
    //     let mut onion_perf_data = OnionPerfData::new().await.unwrap();
    //     onion_perf_data
    //         .create_all_relay_to_relay_combinations()
    //         .await;
    // });

    let mut handles = vec![];
    for i in 1..10 {
        handles.push(gen_thread(i));
    }

    // it waits for all the threads to finish sequentially, we don't care about it right now, all
    // we care about right now is that it makes sure all of them finish
    for handle in handles {
        let _ = handle.join();
    }
    // let tor_client_config = TorClientConfig::default();

    // // Builds net_dir based on latest md_consensus
    // let net_dir = client.dirmgr().timely_netdir().unwrap();
    // let relays: Vec<Relay> = net_dir.relays().into_iter().map(|x| x).collect();

    //let cir_mgr = client.circmgr();
    //for i in 0..500 {
    //let _net_dir = net_dir.clone();
    //let _cir_mgr = cir_mgr.clone();

    //    let p = tokio::task::spawn(async move {
    //        let relays: Vec<Relay> = _net_dir
    //            .relays()
    //            .into_iter()
    //            .map(|x| x.to_owned())
    //            .collect();

    //        let relay_1 = relays[i].clone();
    //        let relay_2 = relays[i + 1].clone();
    //        let c_mgr = _cir_mgr;

    //        let path = tor_circmgr::path::TorPath::new_multihop::<()>(vec![
    //            relay_1.clone(),
    //            relay_2.clone(),
    //        ]);

    //        let circ_parameters = CircParameters::default();
    //        let circ_usage = ChannelUsage::UselessCircuit;

    //        let cir = cir_mgr
    //            .builder()
    //            .build(&path, &circ_parameters, circ_usage)
    //            .await;
    //        match cir {
    //            Ok(c) => {
    //                println!("SUCCESS");
    //                //                            //println!("{:#?}", c.channel());
    //                //                            println!("Time {:?}", std::time::Instant::now().duration_since(t));
    //                //
    //                //                           let x = c
    //                //                               .begin_stream("142.251.46.174", 80, Some(StreamParameters::default()))
    //                //                               .await;
    //                //                           println!("{:#?}", x);
    //                //                           c.terminate();
    //            }
    //            Err(x) => {
    //                println!("{:#?}", x);
    //            }
    //        }
    //    });
    //    //handles.push(p);
    //}

    //        let path = tor_circmgr::path::TorPath::new_multihop::<()>(vec![
    //
    //            relays[i].clone(),
    //            relays[i + 1].clone(),
    //            relays[i + 2].clone(),
    //            relays[i + 3].clone(),
    //            relays[i + 4].clone(),
    //        ]);
    //
    //        let circ_parameters = CircParameters::default();
    //        let circ_usage = ChannelUsage::UselessCircuit;
    //
    //        let cir_mgr = client.circmgr();
    //
    //        let t = std::time::Instant::now();
    //        let cir = cir_mgr
    //            .builder()
    //            .build(&path, &circ_parameters, circ_usage)
    //            .await;
    //        match cir {
    //            Ok(c) => {
    //                //println!("{:#?}", c.channel());
    //                println!("Time {:?}", std::time::Instant::now().duration_since(t));
    //
    //                let x = c
    //                    .begin_stream("142.251.46.174", 80, Some(StreamParameters::default()))
    //                    .await;
    //                println!("{:#?}", x);
    //                c.terminate();
    //            }
    //            Err(x) => {
    //                //println!("{:#?}", x);
    //            }
    //}

    //tokio::time::sleep(Duration::from_secs(100)).await;
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
