use chrono::{Datelike, Duration, TimeZone, Timelike, Utc};
use lzma::LzmaReader;
use reqwest::Url;
use std::fmt::format;
use std::fs::copy;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::time::Instant;

pub struct OnionPerfData {
    /// List of URLs for OnionPerfData
    all_urls: Vec<String>,
}

impl OnionPerfData {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let all_urls = Self::generate_all_urls();
        Self::download_all_onion_perf_data(all_urls.clone()).await;
        Ok(Self { all_urls })
    }

    // Generates a list of URLs for OnionPerf data
    fn generate_all_urls() -> Vec<String> {
        // All hardcoded hosts
        let hosts = vec!["op-de7a", "op-hk6a", "op-hk7a", "op-nl7a", "op-us7a"];

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

        let all_urls : Vec<String> = hosts
            .into_iter()
            .map(|host| {
                format!(
                    "https://collector.torproject.org/recent/onionperf/{}.{}.onionperf.analysis.json.xz",
                    download_date, host
                )
            })
            .collect();

        all_urls
    }

    // TODO : There are tons of Unwrap happening here, replace with with a much more verbose error
    // handling or use error propagation
    pub async fn download_all_onion_perf_data(urls: Vec<String>) {
        // TODO : Decide whether to use the tor network or just make a direct connection and
        // download
        for (index, ref url) in urls.into_iter().enumerate() {
            let current_instant = Instant::now();

            // Download compressed file
            println!("Downloading OnionPerfData from {}", url);
            let url = Url::parse(url).unwrap();
            let response = reqwest::get(url).await.unwrap().bytes().await.unwrap();
            println!(
                "Download time : {:?} | Size(in Bytes) : {}",
                Instant::now().duration_since(current_instant),
                response.len()
            );

            let current_instant = Instant::now();

            // Decompress the compressed file
            println!("Decompressing {index}.json.xz");
            let mut lzma_reader = LzmaReader::new_decompressor(&response[..]).unwrap();
            // Eventhough i saw that OnionPerf JSON data is around 30-40  Mib, assuming upto 100 times of decompression for max safety,
            println!("{:?}", response.len() * 100);
            let mut decompressed_data = Vec::with_capacity(response.len() * 100);
            lzma_reader.read_to_end(&mut decompressed_data).unwrap();
            println!(
                "Decompression time : {:?} | Size(in Bytes) : {}",
                Instant::now().duration_since(current_instant),
                decompressed_data.len()
            );

            // Create the json data file
            let mut dest_file = File::create(format!("{index}.json")).unwrap();
            dest_file.write_all(&decompressed_data).unwrap();
            println!("Created {index}.json \n");
        }
    }
}
