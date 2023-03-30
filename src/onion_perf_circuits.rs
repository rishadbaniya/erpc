use chrono::{Datelike, Duration, TimeZone, Timelike, Utc};
use lzma::LzmaReader;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::format;
use std::fs::copy;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::time::Instant;

// All hardcoded hosts
const HOSTS: [&str; 5] = ["op-de7a", "op-hk6a", "op-hk7a", "op-nl7a", "op-us7a"];

// Data Structure that maps to OnionPerf analysis JSON file (Version : 3.1)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OnionPerfAnalysisData {
    //#[serde(skip_deserializing)]
    data: HashMap<String, Data>,

    #[serde(skip_deserializing)]
    filters: Option<Value>,

    //Document type
    #[serde(rename = "type")]
    _type: String,

    //Document version
    version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Data {
    ///Public IP address of the measuring host
    measurement_ip: String,

    ///Measurement data obtained from client-side TGen logs
    tgen: Value,

    //Metadata obtained from client-side Tor controller logs
    tor: Tor,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tor {
    circuits: HashMap<String, Circuit>,

    #[serde(skip_deserializing)]
    streams: Option<Value>,

    #[serde(skip_deserializing)]
    guards: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Circuit {
    //Circuit identifier, obtained from CIRC and CIRC_MINOR events
    circuit_id: usize,

    //Elapsed seconds until receiving and logging CIRC and CIRC_MINOR events
    elapsed_seconds: Vec<Value>,

    /// Final end time of the circuit, obtained from the log time of the last CIRC CLOSED or CIRC FAILED event, given in seconds since the epoch
    unix_ts_start: f32,

    /// Initial start time of the circuit, obtained from the log time of the CIRC LAUNCHED event, given in seconds since the epoch"
    unix_ts_end: f32,

    failure_reason_local: Option<String>,

    ///Build time in seconds, computed as time elapsed between CIRC LAUNCHED and CIRC BUILT events
    buildtime_seconds: Option<f32>,

    ///Whether this circuit has been filtered out when applying filters in `onionperf filter`
    ///TODO: Figure out what onionperf filter means
    filtered_out: Option<bool>,

    ///Path information
    path: Option<Vec<RelayDetail>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RelayDetail {
    #[serde(rename = "0")]
    fingerprint: String,

    #[serde(rename = "1")]
    time_passed: f32,
}

#[derive(Debug, Clone)]
pub struct OnionPerfRunnerHost {
    /// Name of the host
    host_name: String,

    /// The URL of the OnionPerf data produced by the host
    url: String,

    /// The data of the host
    data: Option<OnionPerfAnalysisData>,

    /// A list of ciruits(with path) marked successful by OnionPerf
    successful_circuits: Vec<Circuit>,

    /// A list of ciruits(with no path) marked failed by OnionPerf
    failed_circuits: Vec<Circuit>,
}

impl OnionPerfRunnerHost {
    fn new<T: AsRef<str>>(host_name: T) -> Self {
        let utc_time = Utc::now();

        // Minimum expected UTC time for OnionPerf Data
        let expected_min_utc_time = Utc
            .with_ymd_and_hms(utc_time.year(), utc_time.month(), utc_time.day(), 2, 30, 00)
            .unwrap();

        // Download date for OnionPerf Data
        //
        // If the current UTC time is greater than the minimum expected UTC time,
        // use the current date for the download date
        let download_date = if utc_time > expected_min_utc_time {
            let one_day_before_utc = utc_time - Duration::days(1);
            format!(
                "{}-{:02}-{:02}",
                one_day_before_utc.year(),
                one_day_before_utc.month(),
                one_day_before_utc.day()
            )
        } else {
            // Otherwise, use the previous day's date for the download date
            let two_day_before_utc = utc_time - Duration::days(1);
            format!(
                "{}-{:02}-{:02}",
                two_day_before_utc.year(),
                two_day_before_utc.month(),
                two_day_before_utc.day()
            )
        };

        let url = format!(
            "https://collector.torproject.org/recent/onionperf/{}.{}.onionperf.analysis.json.xz",
            download_date,
            host_name.as_ref()
        );

        let successful_circuits = vec![];
        let failed_circuits = vec![];

        Self {
            host_name: host_name.as_ref().to_owned(),
            url,
            data: None,
            successful_circuits,
            failed_circuits,
        }
    }

    /// Attempts to download and parse the OnionPerfData of the certain host
    pub async fn download_and_parse_data(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let current_instant = Instant::now();

        // Download compressed file
        println!("Downloading OnionPerfData from {}", self.url);
        let url = Url::parse(&self.url)?;
        let response = reqwest::get(url).await?.bytes().await?;
        println!(
            "Download time : {:?} | Size(in Bytes) : {}",
            Instant::now().duration_since(current_instant),
            response.len()
        );

        let current_instant = Instant::now();

        // Decompress the compressed file
        println!("Decompressing {}.json.xz", self.host_name);
        let mut lzma_reader = LzmaReader::new_decompressor(&response[..])?;

        // Eventhough i saw that OnionPerf JSON data is around 30-40 Mib (Decompressed from around 1 Mb), assuming upto 100 times of decompression for max safety right now
        println!("{:?}", response.len() * 100);
        let mut decompressed_data = Vec::with_capacity(response.len() * 100);
        lzma_reader.read_to_end(&mut decompressed_data)?;
        println!(
            "Decompression time : {:?} | Size(in Bytes) : {}",
            Instant::now().duration_since(current_instant),
            decompressed_data.len()
        );

        // Create the json data file
        let mut dest_file = File::create(format!("{}.json", self.host_name))?;
        dest_file.write_all(&decompressed_data)?;
        println!("Created {}.json \n", self.host_name);

        //  Parsing the JSON file
        let onion_perf_data: OnionPerfAnalysisData = serde_json::from_slice(&decompressed_data)?;
        for x in onion_perf_data
            .data
            .get(&self.host_name)
            .unwrap()
            .tor
            .circuits
            .values()
            .into_iter()
        {
            if let Some(_) = x.path {
                if let Some(_) = x.failure_reason_local {
                    self.failed_circuits.push(x.clone());
                } else {
                    self.successful_circuits.push(x.clone());
                }
            }
        }

        self.data = Some(onion_perf_data);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct OnionPerfData {
    /// List of all OnionPerf host based data
    all_hosts: Vec<OnionPerfRunnerHost>,

    all_sucessful_relay_to_relay_combinations: Vec<(String, String)>,

    all_failed_relay_to_relay_combinations: Vec<(String, String)>,
}

impl OnionPerfData {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut total_successful_circuits = 0;
        let mut total_failed_circuits = 0;

        let mut all_hosts = vec![];

        for host in HOSTS {
            let mut runner_host = OnionPerfRunnerHost::new(host);
            runner_host.download_and_parse_data().await.unwrap();

            total_failed_circuits += runner_host.failed_circuits.len();
            total_successful_circuits += runner_host.successful_circuits.len();

            all_hosts.push(runner_host);
        }

        println!("total failed : {:?}", total_failed_circuits);
        println!("total successful :{:?}", total_successful_circuits);

        let all_sucessful_relay_to_relay_combinations = Vec::new();
        let all_failed_relay_to_relay_combinations = Vec::new();

        Ok(Self {
            all_hosts,
            all_failed_relay_to_relay_combinations,
            all_sucessful_relay_to_relay_combinations,
        })
    }

    pub async fn create_all_relay_to_relay_combinations(&self) {
        let mut _2_paths = 0;
        let mut _3_paths = 0;
        let mut _4_paths = 0;
        let mut _5_paths = 0;

        let mut __2_paths = 0;
        let mut __3_paths = 0;
        let mut __4_paths = 0;
        let mut __5_paths = 0;
        for host in self.all_hosts.iter() {
            for failed_circuit in host.failed_circuits.iter() {
                if let Some(ref path) = failed_circuit.path {
                    if path.len() == 2 {
                        __2_paths += 1;
                    } else if path.len() == 3 {
                        __3_paths += 1;
                    } else if path.len() == 4 {
                        __4_paths += 1;
                    } else if path.len() == 5 {
                        __5_paths += 1;
                    }
                }
            }

            for successful_circuit in host.successful_circuits.iter() {
                if let Some(ref path) = successful_circuit.path {
                    if path.len() == 2 {
                        _2_paths += 1;
                    } else if path.len() == 3 {
                        _3_paths += 1;
                    } else if path.len() == 4 {
                        _4_paths += 1;
                    } else if path.len() == 5 {
                        _4_paths += 1;
                    }
                }
            }
        }

        println!(
            "The total 2 hop circuit in successful circuits are : {}",
            _2_paths
        );
        println!(
            "The total 3 hop circuit in successful circuits are : {}",
            _3_paths
        );
        println!(
            "The total 4 hop circuit in successful circuits are : {}",
            _4_paths
        );
        println!(
            "The total 5 hop circuit in successful circuits are : {}",
            _5_paths
        );

        println!(
            "The total 2 hop circuit in failed circuits are : {}",
            __2_paths
        );
        println!(
            "The total 3 hop circuit in failed circuits are : {}",
            __3_paths
        );
        println!(
            "The total 4 hop circuit in failed circuits are : {}",
            __4_paths
        );
        println!(
            "The total 5 hop circuit in failed circuits are : {}",
            __5_paths
        );
    }
}
