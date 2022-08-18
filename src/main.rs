
use log::{debug, error, log_enabled, info, Level};
use serde::{Deserialize, Deserializer};
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use std::io::BufReader;
use url::Url;
use std::fs::File;
use std::path::PathBuf;
use tokio::sync::mpsc::{error::TryRecvError, Receiver};
use tokio::time::Duration;


#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(deserialize_with = "deserialize_url")]
    lets_encrypt_url: Url,
    monitor: FileMonitor,
    #[serde(deserialize_with = "deserialize_interval", rename = "update_interval_secs")]
    update_interval: Duration,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileMonitor {
    directory: PathBuf,
}

impl FileMonitor {
    fn init(&self) -> Result<(), CredenceError> {
        Ok(std::fs::create_dir_all(&self.directory)?)
    }

    async fn run(self, config: Config, mut rx: Receiver<Option<()>>) {
        loop {
            info!("file monitor started");

            match rx.try_recv() {
                Err(TryRecvError::Empty) => (),
                _ => return (),
            };
            tokio::time::sleep(config.update_interval).await;
        }
    }
}

fn deserialize_interval<'de, D>(data: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let secs: u64 = serde::de::Deserialize::deserialize(data)?;
    Ok(Duration::from_secs(secs))
}

fn deserialize_url<'de, D>(data: D) -> Result<Url, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = serde::de::Deserialize::deserialize(data)?;
    Url::parse(&s).map_err(serde::de::Error::custom)
}

#[derive(Debug)]
enum CredenceError {
    IO(std::io::Error),
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {

    env_logger::init();

    let args = clap::Command::new("credence")
        .arg(clap::Arg::with_name("config-check")
            .long("config-check")
            .help("Whether to just check the config and exit"))
        .arg(clap::Arg::with_name("config-file")
            .long("config-file")
            .help("Path to config file")
            .takes_value(true));
    

    let m = args.get_matches();

    let file_path = m.value_of("config-file").unwrap();
    let file = File::open(&file_path).unwrap();
    let reader = BufReader::new(file);
    let config: Config = serde_json::from_reader(reader).unwrap();

    if m.is_present("config-check") {
        std::process::exit(0);
    }

    let (tx, rx) = tokio::sync::mpsc::channel(10);

    // for now we have only one monitor type, so no branch out or magic here
    config.monitor.init().unwrap();
    let config_clone = config.clone();
    let monitor = tokio::spawn(config.monitor.run(config_clone, rx));

    let mut stream = signal(SignalKind::terminate()).unwrap();
    let signal_handler = tokio::spawn(async move {
        let sig = stream.recv().await;
        tx.send(sig).await.unwrap();
    });
    let _ = tokio::join!(monitor, signal_handler);
}

impl std::convert::From<std::io::Error> for CredenceError {
    fn from(inner: std::io::Error) -> CredenceError {
        CredenceError::IO(inner)
    }
}