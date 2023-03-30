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
    failure_reason_local: Option<String>,

    filtered_out: bool,

    path: Option<Value>,
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
        }

        println!("total failed : {:?}", total_failed_circuits);
        println!("total successful :{:?}", total_successful_circuits);

        Ok(Self { all_hosts })
    }
}
