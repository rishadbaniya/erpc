// NOTE: THIS CODE IS UNOPTIMIZED IN SO MANYYYYYYYYYY WAYS, GONNA OPTIMIZE LATER ON LETS PRODUCE A
// WORKING EXAMPLE RIGHT NOW

mod onion_perf_circuits;
use arti_client::config::CfgPath;
use clap::Parser;
// TODO : Check for Bridge Rlelay Partitioning from other(non bridge relay)
use arti_client::config::circ::PathConfigBuilder;
use arti_client::TorClient;
use arti_client::TorClientConfig;
use chrono::DateTime;
use chrono::Utc;
use onion_perf_circuits::OnionPerfData;
use rand::seq::IteratorRandom;
use std::fs;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::vec;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tor_chanmgr::ChannelUsage;
use tor_circmgr::path::TorPath;
use tor_circmgr::CircMgr;
use tor_netdir::MdReceiver;
use tor_netdir::PartialNetDir;
use tor_netdir::Relay;
use tor_netdoc::doc::netstatus::Consensus;
use tor_netdoc::doc::netstatus::MdConsensus;
use tor_netdoc::doc::netstatus::RouterStatus;
use tor_proto::circuit::CircParameters;
use tor_proto::circuit::ClientCirc;
use tor_proto::stream::StreamParameters;
use tor_rtcompat::Runtime;

//#[command(author, version, about, long_about = None)]
#[derive(Debug, Parser)]
struct Args {
    /// Wait duration in seconds
    #[arg(short, long, default_value_t = 5)]
    gap_duration: u16,

    /// Total no of relays to test randomly from the given data type of the value
    #[arg(short, long, default_value_t = 400)]
    relay_count: usize,

    /// Total no of relays to test randomly from the given data type of the value
    #[arg(short, long, default_value_t = false)]
    should_test_onion_perf: bool,

    /// Total no of threads to spin i.e total arti_clients you wannaa spin
    /// Please make it multiple of 5
    #[arg(short, long, default_value_t = 40)]
    no_of_threads_to_spin: usize,
}

#[derive(Debug, Clone)]
struct LogData {
    /// Fingerprint of the source relay
    source_relay: String,
    /// Fingerprint of the destination relay
    destination_relay: String,
    /// Time the attempt was made
    time: DateTime<Utc>,
    /// If error then T else None
    /// I've stringified the error right now
    err: Option<String>,
}

impl LogData {
    pub fn to_csv(&self) -> String {
        let err = match self.err {
            Some(ref e) => e.clone(),
            None => "No error".to_owned(),
        };
        format!(
            "{},{},{},{}",
            self.source_relay,
            self.destination_relay,
            self.time.to_string(),
            err
        )
    }
}
impl Default for LogData {
    fn default() -> Self {
        Self {
            source_relay: "".to_owned(),
            destination_relay: "".to_owned(),
            time: Utc::now(),
            err: None,
        }
    }
}

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

fn gen_thread(
    i: usize,
    index_of_random_relays: Vec<u32>,
    total_sub_nodes: usize,
    storage: Arc<Mutex<Vec<LogData>>>,
    no_of_relays_to_be_tested_globally: usize,
) -> std::thread::JoinHandle<()> {
    let handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async move {
            // The batch of work given to it by the main thread

            // NO_OF_RELAYS_TO_TEST is the hardcoded no of relays to test
            // let's call threads as sub nodes haha
            let no_of_relays_to_test = no_of_relays_to_be_tested_globally / total_sub_nodes;
            let from = i * no_of_relays_to_test;
            let to = (i * no_of_relays_to_test) + no_of_relays_to_test;

            let my_batch_to_work_on = &index_of_random_relays[from..to];

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

            for index in my_batch_to_work_on {
                for j in &index_of_random_relays {
                    // Only make circuits if their index don't match, else how tf are we making
                    // circuits with ownself
                    if index != j {
                        let relay_1 = relays[*index as usize].clone();
                        let relay_2 = relays[*j as usize].clone();
                        let path = tor_circmgr::path::TorPath::new_multihop::<()>(vec![
                            relays[*index as usize].clone(), // Source relay
                            relays[*j as usize].clone(),     // Destination relay
                        ]);
                        let circ_parameters = CircParameters::default();
                        let circ_usage = ChannelUsage::UselessCircuit;

                        let cir = cir_mgr
                            .builder()
                            .build(&path, &circ_parameters, circ_usage)
                            .await;

                        // If error then just simply give me the stringified version of that error
                        let err: Option<String> = match cir {
                            Ok(c) => {
                                // SHUTDOWN control message
                                c.terminate();
                                None
                            }
                            Err(e) => Some(e.to_string()),
                        };

                        let log_data = LogData {
                            source_relay: relay_1.rs().rsa_identity().to_string().replace("$", ""),
                            destination_relay: relay_2
                                .rs()
                                .rsa_identity()
                                .to_string()
                                .replace("$", ""),
                            time: Utc::now(),
                            err,
                        };

                        storage.lock().await.push(log_data.clone());
                        sleep(Duration::from_secs(5)).await;
                        println!("{}", log_data.to_csv());
                    }
                }
            }
        });
    });
    handle
}

// NOTE : I've used 100 clients here because as of right now i was unable to figure out a way to do
// it in single thread through tokio::spawn (life time issues persisted, which wouldn't let me pass
// CircMgr across tasks)
fn main() {
    let args = Args::parse();
    // Checks for 2 hop circuits in onion perf
    if args.should_test_onion_perf {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let mut onion_perf_data = OnionPerfData::new().await.unwrap();
            onion_perf_data
                .create_all_relay_to_relay_combinations()
                .await;
        });
    }

    //Let's keep it multiple of 5 right now for test purpose
    let log_storage: Arc<Mutex<Vec<LogData>>> = Arc::default();
    let no_of_threads = args.no_of_threads_to_spin;
    let mut rng = rand::thread_rng();

    // We haven't counted relay here, lets assume upper bound of 6500, whcih means i won't be
    // selecting the randoms higher than 6500 index, no worries its just a test :)
    let random_relays: Vec<u32> = (0..=6500)
        .choose_multiple(&mut rng, args.relay_count)
        .into_iter()
        .collect();

    let mut handles = vec![];
    let no_of_relays_to_be_tested_globally = args.relay_count;
    for i in 0..no_of_threads {
        handles.push(gen_thread(
            i,
            random_relays.clone(),
            no_of_threads,
            log_storage.clone(),
            no_of_relays_to_be_tested_globally,
        ));
    }

    // it waits for all the threads to finish sequentially, we don't care about it right now, all
    // we care about right now is that it makes sure all of them finish
    for handle in handles {
        let _ = handle.join();
    }

    let file = std::fs::File::create("data.csv").expect("Error creating file");
    let mut writer = std::io::BufWriter::new(file);

    for log_data in &(*log_storage.blocking_lock()) {
        let _ = writer.write_fmt(format_args!("{}\n", log_data.to_csv()));
    }
}
