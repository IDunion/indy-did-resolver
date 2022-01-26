use futures_executor::block_on;
use indy_vdr::pool::{LocalPool, Pool, PoolBuilder, PoolTransactions, PreparedRequest, RequestResult, TimingResult};
use std::fs;
use std::collections::HashMap;
use indy_vdr::common::error::VdrResult;
use indy_vdr::pool::helpers::perform_ledger_request;
use indy_didresolver::did::did_parse;
use indy_didresolver::did_document::DidDocument;
use indy_didresolver::responses::GetNymResult;
use indy_vdr::utils::did::DidValue;
use regex::Regex;
use serde_json::Value;
use indy_didresolver::error::{DidIndyError, DidIndyResult};
use rouille::Response;
use serde_json::Value::Null;

#[allow(dead_code)]
struct Ledger {
    name: String,
    //TODO: Source(local,github)
    pool: LocalPool
}

//did:indy:idunion:BDrEcHc8Tb4Lb2VyQZWEDE
//did:indy:eesdi:H1iHEynabfar9mp4uprW6W


fn main() {

    rouille::start_server("0.0.0.0:8080", move |request| {
        let url = request.url();
        println!("incoming request: {}",url);
        let request_regex = Regex::new("/1.0/identifiers/(.*)").expect("Error in the DID Regex Syntax");

        let captures = request_regex.captures(&url);
        match captures {
            Some(cap) => {
                let did = cap.get(1).unwrap().as_str();
                match process_request( did) {
                    Ok(did_document) => {
                        Response::text(did_document)
                    }
                    Err(_) => {
                        Response::text("404").with_status_code(404)
                    }
                }
                //let did_document = process_request( did);
                //Response::text(did_document)
            }
            None => {
                Response::text("400").with_status_code(400)
            }
        }
    });

    // loop {
    //     println!("Please provide the requested DID...");
    //
    //     let mut request = String::new();
    //     io::stdin().read_line(&mut request).expect("Failed to read line");
    //
    //     process_request(request.as_str());
    // }

}

fn process_request(request: &str) -> DidIndyResult<String> {

    let mut ledgers: HashMap<String, Ledger> = HashMap::new();
    let entries = fs::read_dir("./genesis-files");
    match entries {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        let name = entry.file_name().into_string().unwrap();
                        println!("Processing Genesis file {}", name);
                        let txns = PoolTransactions::from_json_file(entry.path()).unwrap();
                        // Create a PoolBuilder instance
                        let pool_builder = PoolBuilder::default().transactions(txns).unwrap();
                        // Convert into a thread-local Pool instance
                        let pool = pool_builder.into_local().unwrap();
                        let ledger = Ledger {
                            name: name.clone(),
                            pool,
                        };
                        ledgers.insert(name, ledger);
                    },
                    Err(_) => println!("Error reading genesis directory"),
                }
            }
        },
        Err(_) => println!("Error reading genesis directory"),
    }

    println!("Successfully imported {} Indy Ledgers from Files", ledgers.len());

    let did = match did_parse(request.trim()){
        Ok(did) => {
            did
        },
        Err(DidIndyError) => {
            return Err(DidIndyError);
        }
    };

    let ledger= match ledgers.get(&did.namespace) {
        Some(ledger) => {
            ledger
        }
        None => {
            println!("Requested Indy Namespace \"{}\" unknown", &did.namespace);
            return Err(DidIndyError);
        }
    };

    let pool = &ledger.pool;
    let builder = pool.get_request_builder();
    let did_value = DidValue::new(&did.id, Option::None);
    let request = builder.build_get_nym_request(Option::None, &did_value).unwrap();
    let (result, _timing) = block_on(request_transaction(pool, request)).unwrap();
    let ledger_data = match result {
        RequestResult::Reply(data) => {
            data
        }
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

    let did_document = DidDocument::new(&did.namespace,&get_nym.dest,&get_nym.verkey);
    let json = did_document.to_string();

    println!("DID Document: {}", json);

    return Ok(json)
}

pub async fn request_transaction<T: Pool>(
    pool: &T,
    request : PreparedRequest,
) -> VdrResult<(RequestResult<String>, Option<TimingResult>)> {
    perform_ledger_request(pool, &request).await
}

