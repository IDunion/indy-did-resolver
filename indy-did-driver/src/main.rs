use futures_executor::block_on;
use git2::Repository;
use indy_didresolver::did::did_parse;
use indy_didresolver::did_document::DidDocument;
use indy_didresolver::error::{DidIndyError, DidIndyResult};
use indy_didresolver::responses::GetNymResult;
use indy_vdr::common::error::VdrResult;
use indy_vdr::pool::helpers::perform_ledger_request;
use indy_vdr::pool::{
    Pool, PoolBuilder, PoolTransactions, PreparedRequest, RequestResult, SharedPool, TimingResult,
};
use indy_vdr::utils::did::DidValue;
use regex::Regex;
use rouille::Response;
use serde_json::Value;
use serde_json::Value::Null;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

static GITHUB_NETWORKS: &str = "https://github.com/domwoe/networks";
static GENESIS_FILE_NAME: &str = "pool_transactions_genesis.json";

type Ledgers = HashMap<String, SharedPool>;

//did:indy:idunion:BDrEcHc8Tb4Lb2VyQZWEDE
//did:indy:eesdi:H1iHEynabfar9mp4uprW6W

fn main() {
    let ledgers = init_ledgers("");

    rouille::start_server("0.0.0.0:8081", move |request| {
        let url = request.url();
        println!("incoming request: {}", url);
        let request_regex =
            Regex::new("/1.0/identifiers/(.*)").expect("Error in the DID Regex Syntax");

        let captures = request_regex.captures(&url);
        match captures {
            Some(cap) => {
                let did = cap.get(1).unwrap().as_str();
                match process_request(did, &ledgers) {
                    Ok(did_document) => Response::text(did_document),
                    Err(_) => Response::text("404").with_status_code(404),
                }
                //let did_document = process_request( did);
                //Response::text(did_document)
            }
            None => Response::text("400").with_status_code(400),
        }
    });
}

fn init_ledgers(source: &str) -> Ledgers {
    let mut ledgers: Ledgers = HashMap::new();
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

                let (ledger_prefix, txns) = match sub_namespace {
                    Some(sub_namespace) => (
                        // Todo: Change to '.' potentially
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

                let pool_builder = PoolBuilder::default().transactions(txns).unwrap();
                let pool = pool_builder.into_shared().unwrap();

                ledgers.insert(ledger_prefix, pool);
            }
        }
    }

    println!("{:?}", ledgers.keys());
    ledgers
}

fn process_request(request: &str, ledgers: &Ledgers) -> DidIndyResult<String> {
    let did = match did_parse(request.trim()) {
        Ok(did) => did,
        Err(DidIndyError) => {
            return Err(DidIndyError);
        }
    };

    let pool = match ledgers.get(&did.namespace) {
        Some(ledger) => ledger,
        None => {
            println!("Requested Indy Namespace \"{}\" unknown", &did.namespace);
            return Err(DidIndyError);
        }
    };

    let builder = pool.get_request_builder();
    let did_value = DidValue::new(&did.id, Option::None);
    let request = builder
        .build_get_nym_request(Option::None, &did_value)
        .unwrap();
    let (result, _timing) = block_on(request_transaction(pool, request)).unwrap();
    let ledger_data = match result {
        RequestResult::Reply(data) => data,
        RequestResult::Failed(error) => {
            println!("Error requesting DID from ledger, {}", error.to_string());
            return Err(DidIndyError);
        }
    };

    println!("Request successful: {}", ledger_data);
    let v: Value = serde_json::from_str(&ledger_data).unwrap();
    println!("result: {:?}", v);
    let data: &Value = &v["result"]["data"];
    println!("data: {:?}", data);
    if *data == Null {
        return Err(DidIndyError);
    }
    let get_nym: GetNymResult = serde_json::from_str(data.as_str().unwrap()).unwrap();
    println!("get_nym: {:?}", get_nym);

    let did_document = DidDocument::new(&did.namespace, &get_nym.dest, &get_nym.verkey);
    let json = did_document.to_string();

    println!("DID Document: {}", json);

    return Ok(json);
}

pub async fn request_transaction<T: Pool>(
    pool: &T,
    request: PreparedRequest,
) -> VdrResult<(RequestResult<String>, Option<TimingResult>)> {
    perform_ledger_request(pool, &request).await
}
