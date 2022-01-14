

pub fn

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