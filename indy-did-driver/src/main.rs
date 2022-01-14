use futures_executor::block_on;
use indy_vdr::pool::{LocalPool, Pool, PoolBuilder, PoolTransactions, PreparedRequest, RequestResult, TimingResult};
use std::{fmt, fs, io, process};
use std::collections::HashMap;
use std::ops::Add;
use indy_didresolver::did;
use indy_vdr::common::error::VdrResult;
use indy_vdr::pool::helpers::perform_ledger_request;
use indy_didresolver::did::{Did, did_parse};
use indy_didresolver::did_document::{DidDocument, Ed25519VerificationKey2018};
use indy_didresolver::responses::GetNymResult;
use indy_vdr::utils::did::DidValue;
use serde_json::Value;

struct Ledger {
    name: String,
    pool: LocalPool
}

//did:indy:idunion:BDrEcHc8Tb4Lb2VyQZWEDE
//did:indy:eesdi:H1iHEynabfar9mp4uprW6W


fn main() {

    let mut ledgers : HashMap<String, Ledger> = HashMap::new();
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

    println!("Please provide the requested DID...");

    let mut request = String::new();
    io::stdin().read_line(&mut request).expect("Failed to read line");
    //request.push_str("did:indy:idunion:BDrEcHc8Tb4Lb2VyQZWEDE");

    let mut ledger_data = String::new();

    match did::did_parse(request.trim()){
        Ok(did) => {
            match ledgers.get(&did.namespace) {
                Some(ledger) => {
                    let pool = &ledger.pool;
                    let builder = pool.get_request_builder();
                    let did_value = DidValue::new(&did.id, Option::None);
                    let request = builder.build_get_nym_request(Option::None, &did_value).unwrap();
                    let (result, _timing) = block_on(request_transaction(pool, request)).unwrap();
                    match result {
                        RequestResult::Reply(data) => {
                            ledger_data = data;
                        }
                        RequestResult::Failed(error) => {
                            println!("Error requesting DID from ledger, {}", error.to_string());
                            process::exit(1);
                        }
                    }
                }
                None => {
                    println!("Requested Indy Namespace \"{}\" unknown", &did.namespace);
                    process::exit(1);
                }
            }
        },
        Err(DidIndyError) => {
            panic!(DidIndyError);
        }
    }

    println!("Request successful: {}", ledger_data);
    let v: Value = serde_json::from_str(&ledger_data).unwrap();
    println!("result: {:?}", v);
    let data: &Value = &v["result"]["data"];
    println!("data: {:?}", data);
    let get_nym: GetNymResult = serde_json::from_str(data.as_str().unwrap()).unwrap();
    println!("get_nym: {:?}", get_nym);

    let did_document = DidDocument {
        id: "did:indy:idunion:".to_string().add(&get_nym.dest.to_string()),
        verification_method: vec![Ed25519VerificationKey2018 {
            id: "did:indy:idunion:".to_string().add(&get_nym.dest.to_string()).add("#keys-1"),
            type_: "Ed25519VerificationKey2018".to_string(),
            controller: "did:indy:idunion:".to_string() + &get_nym.dest.to_string(),
            public_key_base58: get_nym.verkey,
        }],
        authentication: vec![
            "did:indy:idunion".to_string() + &get_nym.dest.to_string().add("#keys-1")
        ]
    };

    let json = serde_json::to_string_pretty(&did_document).unwrap();
    println!("DID Document: {}", json);

}

pub async fn request_transaction<T: Pool>(
    pool: &T,
    request : PreparedRequest,
) -> VdrResult<(RequestResult<String>, Option<TimingResult>)> {
    perform_ledger_request(pool, &request).await
}

