use futures_executor::block_on;
use git2::Repository;
use indy_didresolver::did::DidUrl;
use indy_didresolver::error::{DidIndyError, DidIndyResult};
use indy_didresolver::resolver::Resolver;
use indy_vdr::pool::{helpers::perform_refresh, PoolBuilder, PoolTransactions, SharedPool};
use regex::Regex;
use rouille::Response;

use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
#[macro_use]
extern crate log;

static POOL_SIZE: Option<usize> = Some(32);
type Resolvers = HashMap<String, Resolver<SharedPool>>;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Port to expose
    #[clap(short = 'p', long = "port", default_value_t = 8080)]
    port: u32,
    /// source to use, allowed values are path or github
    #[clap(short = 's', long = "source", default_value = "")]
    source: String,
    /// github repository for registered networks
    #[clap(
        short = 'n',
        long = "github-network",
        default_value = "https://github.com/IDunion/indy-did-networks"
    )]
    github_networks: String,
    /// Pool transaction genesis filename
    #[clap(
        short = 'f',
        long = "genesis-filename",
        default_value = "pool_transactions_genesis.json"
    )]
    genesis_filename: String,
}

fn main() {
    let args = Args::parse();
    let port = &args.port.to_string();
    env_logger::init();
    info!("Starting the indy-did-driver with the following configuration:");
    info!("{:?}", args);

    let resolvers = init_resolvers(args);

    rouille::start_server_with_pool(String::from("0.0.0.0:") + port, POOL_SIZE, move |request| {
        let url = request.url();
        debug!("incoming request: {}", url);
        let request_regex = Regex::new("/1.0/identifiers/(.*)").unwrap();

        let captures = request_regex.captures(&url);
        if let Some(cap) = captures {
            let did = cap.get(1).unwrap().as_str();

            match process_request(did, &resolvers) {
                Ok(result) => {
                    info!("Serving for {}", &url);
                    debug!("Serving DID Doc: {:?}", result);
                    Response::text(result)
                }
                Err(err) => {
                    error!("404: {:?}", err);
                    Response::text("404").with_status_code(404)
                }
            }
        } else {
            info!("400: unrecognized path: {}", &url);
            Response::text("400").with_status_code(400)
        }
    });
}

fn init_resolvers(args: Args) -> Resolvers {
    let mut resolvers: Resolvers = HashMap::new();
    let source = args.source;
    let path = if source == "github" || source.is_empty() {
        info!("Obtaining network information from github");
        // Delete folder if it exists and reclone repo
        fs::remove_dir_all("github").ok();
        let repo = Repository::clone(args.github_networks.as_str(), "github")
            .expect("Could not clone network repository.");
        repo.path().parent().unwrap().to_owned()
    } else if source.starts_with("http:") || source.starts_with("https:") {
        unimplemented!("Download of genesis files from custom location is not supported");
    } else {
        info!("Obtaining network information from local path {}", source);
        PathBuf::from(source)
    };

    let entries = fs::read_dir(path).expect("Could not read path");
    for entry in entries {
        let entry = entry.unwrap();
        // filter hidden directories starting with "."
        if !entry.file_name().to_str().unwrap().starts_with(".")
            && entry.metadata().unwrap().is_dir()
        {
            let namespace = entry.path().file_name().unwrap().to_owned();
            let sub_entries = fs::read_dir(entry.path()).unwrap();
            for sub_entry in sub_entries {
                let sub_entry_path = sub_entry.unwrap().path();
                let sub_namespace = if sub_entry_path.is_dir() {
                    sub_entry_path.file_name()
                } else {
                    None
                };
                let (ledger_prefix, genesis_txns) = match sub_namespace {
                    Some(sub_namespace) => (
                        format!(
                            "{}:{}",
                            namespace.to_str().unwrap(),
                            sub_namespace.to_str().unwrap()
                        ),
                        PoolTransactions::from_json_file(
                            sub_entry_path.join(args.genesis_filename.as_str()),
                        )
                        .unwrap(),
                    ),
                    None => (
                        String::from(namespace.to_str().unwrap()),
                        PoolTransactions::from_json_file(
                            entry.path().join(args.genesis_filename.as_str()),
                        )
                        .unwrap(),
                    ),
                };
                debug!("Initializing pool for {}", ledger_prefix);

                let pool_builder = PoolBuilder::default()
                    .transactions(genesis_txns.clone())
                    .unwrap();
                let mut pool = pool_builder.into_shared().unwrap();

                // Refresh pool to get current validator set
                debug!("Refreshing pool for {}", ledger_prefix);
                let (txns, _timing) = block_on(perform_refresh(&pool)).unwrap();

                pool = if let Some(txns) = txns {
                    let builder = {
                        let mut pool_txns = genesis_txns;
                        pool_txns.extend_from_json(&txns).unwrap();
                        PoolBuilder::default()
                            .transactions(pool_txns.clone())
                            .unwrap()
                    };
                    builder.into_shared().unwrap()
                } else {
                    pool
                };

                resolvers.insert(ledger_prefix, Resolver::new(pool));
            }
        }
    }

    info!("Initialized networks: {:?}", resolvers.keys());
    resolvers
}

fn process_request(request: &str, resolvers: &Resolvers) -> DidIndyResult<String> {
    let did = DidUrl::from_str(request)?;
    let resolver = if let Some(resolver) = resolvers.get(&did.namespace) {
        resolver
    } else {
        error!("Requested Indy Namespace \"{}\" unknown", &did.namespace);
        return Err(DidIndyError::NamespaceNotSupported);
    };

    if did.path.is_none() {
        resolver.resolve(request)
    } else {
        resolver.dereference(request)
    }
}
