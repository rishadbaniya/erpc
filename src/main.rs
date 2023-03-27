use arti_client::TorClient;
use arti_client::TorClientConfig;
#[tokio::main]
async fn main() {
    // Generate the Microdescriptor consensus document at the default storage provided by storage
    // config
    // TODO: Make use of it to explore the Relay
    let config = TorClientConfig::default();

    let tor_client = TorClient::create_bootstrapped(config).await.unwrap();
}
