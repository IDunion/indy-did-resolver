use futures_executor::block_on;
use git2::Repository;
use indy_didresolver::did::DidUrl;
use indy_didresolver::error::{DidIndyError, DidIndyResult};
use indy_didresolver::resolver::Resolver;
use indy_vdr::pool::{helpers::perform_refresh, PoolBuilder, PoolTransactions, SharedPool};
use regex::Regex;
use rouille::Response;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

static PORT: &str = "8080";
static POOL_SIZE: Option<usize> = Some(32);

static GITHUB_NETWORKS: &str = "https://github.com/domwoe/networks";
static GENESIS_FILE_NAME: &str = "pool_transactions_genesis.json";

type Resolvers = HashMap<String, Resolver<SharedPool>>;

//did:indy:idunion:BDrEcHc8Tb4Lb2VyQZWEDE
//did:indy:eesdi:H1iHEynabfar9mp4uprW6W

fn main() {
    let resolvers = init_resolvers("");

    rouille::start_server_with_pool(String::from("0.0.0.0:") + PORT, POOL_SIZE, move |request| {
        let url = request.url();
        println!("incoming request: {}", url);
        let request_regex = Regex::new("/1.0/identifiers/(.*)").unwrap();

        let captures = request_regex.captures(&url);
        if let Some(cap) = captures {
            let did = cap.get(1).unwrap().as_str();

            match process_request(did, &resolvers) {
                Ok(result) => Response::text(result),
                Err(err) => {
                    println!("{:?}", err);
                    Response::text("404").with_status_code(404)
                }
            }
        } else {
            Response::text("400").with_status_code(400)
        }
    });
}

fn init_resolvers(source: &str) -> Resolvers {
    let mut resolvers: Resolvers = HashMap::new();
    let path = if source == "github" || source.is_empty() {
        // Delete folder if it exists and reclone repo
        fs::remove_dir_all("github").ok();
        let repo = Repository::clone(GITHUB_NETWORKS, "github")
            .expect("Could not clone network repository.");
        repo.path().parent().unwrap().to_owned()
    } else if source.starts_with("http:") || source.starts_with("https:") {
        unimplemented!("Download of genesis files from custom location is not supported");
    } else {
        PathBuf::from(source)
    };

    let entries = fs::read_dir(path).expect("Could not read path");
    for entry in entries {
        let entry = entry.unwrap();
        // filter hidden directories starting with "."
        if !entry.file_name().to_str().unwrap().starts_with(".") {
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
                        PoolTransactions::from_json_file(sub_entry_path.join(GENESIS_FILE_NAME))
                            .unwrap(),
                    ),
                    None => (
                        String::from(namespace.to_str().unwrap()),
                        PoolTransactions::from_json_file(entry.path().join(GENESIS_FILE_NAME))
                            .unwrap(),
                    ),
                };

                let pool_builder = PoolBuilder::default()
                    .transactions(genesis_txns.clone())
                    .unwrap();
                let mut pool = pool_builder.into_shared().unwrap();

                // Refresh pool to get current validator set
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

    println!("{:?}", resolvers.keys());
    resolvers
}

fn process_request(request: &str, resolvers: &Resolvers) -> DidIndyResult<String> {
    let did = DidUrl::from_str(request)?;
    let resolver = if let Some(resolver) = resolvers.get(&did.namespace) {
        resolver
    } else {
        println!("Requested Indy Namespace \"{}\" unknown", &did.namespace);
        return Err(DidIndyError::NamespaceNotSupported);
    };

    if did.path.is_none() {
        resolver.resolve(request)
    } else {
        resolver.dereference(request)
    }
}
