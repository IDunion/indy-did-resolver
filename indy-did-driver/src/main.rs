use futures_executor::block_on;
use indy_vdr::pool::{LocalPool, Pool, PoolBuilder, PoolTransactions, PreparedRequest, RequestResult, TimingResult};
use std::{fs, io, process};
use std::collections::HashMap;
use std::ops::Add;
use indy_vdr::common::error::VdrResult;
use indy_vdr::pool::helpers::perform_ledger_request;
use indy_didresolver::did::{did_parse, Did};
use indy_didresolver::did_document::{DidDocument, Ed25519VerificationKey2018};
use indy_didresolver::responses::GetNymResult;
use indy_vdr::utils::did::DidValue;
use serde_json::Value;
use indy_didresolver::error::{DidIndyError, DidIndyResult};
use rouille::Request;
use rouille::Response;

struct Ledger {
    name: String,
    pool: LocalPool
}

//did:indy:idunion:BDrEcHc8Tb4Lb2VyQZWEDE
//did:indy:eesdi:H1iHEynabfar9mp4uprW6W


fn main() {
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

    loop {
        println!("Please provide the requested DID...");

        let mut request = String::new();
        io::stdin().read_line(&mut request).expect("Failed to read line");

        process_request(&ledgers, request);
    }

}

fn process_request(ledgers: &HashMap<String, Ledger>, request: String) -> String {

    let did = match did_parse(request.trim()){
        Ok(did) => {
            did
        },
        Err(DidIndyError) => {
            panic!(DidIndyError);
        }
    };

    let ledger= match ledgers.get(&did.namespace) {
        Some(ledger) => {
            ledger
        }
        None => {
            println!("Requested Indy Namespace \"{}\" unknown", &did.namespace);
            panic!(DidIndyError);
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
            panic!(DidIndyError);
        }
    };

    println!("Request successful: {}", ledger_data);
    let v: Value = serde_json::from_str(&ledger_data).unwrap();
    println!("result: {:?}", v);
    let data: &Value = &v["result"]["data"];
    println!("data: {:?}", data);
    let get_nym: GetNymResult = serde_json::from_str(data.as_str().unwrap()).unwrap();
    println!("get_nym: {:?}", get_nym);

    let did_document = DidDocument::new(&did.namespace,&get_nym.dest,&get_nym.verkey);
    let json = did_document.to_string();

    println!("DID Document: {}", json);

    return json



}

pub async fn request_transaction<T: Pool>(
    pool: &T,
    request : PreparedRequest,
) -> VdrResult<(RequestResult<String>, Option<TimingResult>)> {
    perform_ledger_request(pool, &request).await
}

