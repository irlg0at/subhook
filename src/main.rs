use reqwest::{get, Client, Response};
use clokwerk::{AsyncScheduler, TimeUnits, Job};
use rusqlite::Connection;
use std::collections::{HashSet, HashMap};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use clap::Parser;
use std::sync::Arc;

mod db;
mod domains;
use domains::Subdomains;

const SHODANURL: &str = "https://api.shodan.io/dns/domain/";

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CLI {
    #[arg(long = "platform", short = 'p', env = "PLATFORM")]
    platform: String,

    #[arg(long = "webhook", short = 'w', env = "WEBHOOK_URL")]
    webhook_url: String,

    #[arg(long = "database", short = 'D', env = "DATABASE_PATH", default_value = "/data/subhook.sqlite")]
    database_path: std::path::PathBuf,
 
    #[arg(long = "keyfile",short = 'k', env = "KEYFILE", default_value = "/run/secrets/shodan_api_key")]
    keyfile: String,
    
    #[arg(long = "domains",short = 'd', env = "DOMAINS", default_value = "./domains.txt")]
    domains: String,
    
    #[arg(long = "debug", env = "DEBUG")]
    debug: bool,
}

#[tokio::main]
async fn main() {
    let time = "08:00";
    let mut scheduler = AsyncScheduler::with_tz(chrono::Utc);
    let args = Arc::new(CLI::parse()); 

    let run_update = {
        let args = Arc::clone(&args); 
        move || {
            let args = Arc::clone(&args); 
            async move {
                let shodan_api_key: String =
                    fs::read_to_string(&args.keyfile).expect("Not able to read Shodan secret");
                let domains =
                    BufReader::new(File::open(&args.domains).expect("Not able to read domains file."));
                let db_path = Path::new(&args.database_path);
                if !db_path.exists() {
                    db::initialize_db(db_path).expect("Could not create or reach database.");
                    println!("Database created!");
                }

                println!("Running update of database...");
                update(domains, shodan_api_key, &args.database_path, &args.webhook_url, &args.platform,&args.debug).await; 
                println!("Update done!");
            }
        }
    };
    
    if args.debug {
        run_update().await;
    }
    scheduler.every(1.day()).at(time).run(run_update);

    loop {
        scheduler.run_pending().await;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

async fn update(domains: BufReader<File>, shodan_api_key: String, db_path: &Path, webhook_url:&str, platform:&str, debug: &bool) -> () {

    let mut db_connection =
        Connection::open(db_path).expect("Could not open database, but path exists");

    for domain in domains.lines() {
        let domain = match domain {
            Ok(domain) => domain,
            Err(domain) => {
                eprint!("Failed to read domain on line {domain}");
                continue;
            }
        };

        let subdomains = match get_shodan_subdomains(domain.to_string(), &shodan_api_key).await {
            Ok(json) => json,
            Err(e) => {
                eprintln!("Skipping domain {}, cause of {}", domain, e);
                continue;
            }
        };

        let json: Subdomains = match serde_json::from_str(&subdomains) {
            Ok(subdomains) => subdomains,
            Err(e) => {
                eprint!("Failed to parse JSON for {domain}: {e}");
                continue;
            }
        };

        let exists: bool = match db_connection.query_row(
            "SELECT EXISTS(SELECT name FROM domain WHERE name = ?)",
            (&json.domain,),
            |row| row.get(0),
        ) {
            Ok(bool) => bool,
            Err(e) => {
                eprintln!("Fault occurred when checking existence of {domain}: {e}");
                continue;
            }
        };

        if !exists {
            match db::db_add_domain(&json, &mut db_connection) {
                Ok(()) => println!("Added domain {domain} to database"),
                Err(e) => {
                    eprintln!("Failed to add {domain}: {e}");
                    continue;
                }
            };
        }

        let db = match db::get_db_subdomains(&domain, &mut db_connection) {
            Ok(set) => set,
            Err(e) => {
                eprintln!("Failed to get database subdomains for {domain}: {e}");
                continue;
            }
        };

        let shodan = json.subdomains.into_iter().collect();

        let diff = diff_subdomains(&db, &shodan);

        match db::db_add_subdomains(&domain, &diff.0, true, &mut db_connection) {
            Ok(()) => (),
            Err(e) => eprintln!("Failed to add new subdomains for {domain}: {e}"),
        };
        
        match db::db_add_subdomains(&domain, &diff.1, false, &mut db_connection) {
            Ok(()) => (),
            Err(e) => eprintln!("Failed to update inactive subdomains for {domain}: {e}"),
        };

        if !diff.0.is_empty() || *debug { 
            let endpoint = match platform {
                "slack" => {
                    format!("{}",webhook_url)
                },

                "discord" => {
                    format!("{}/slack",webhook_url)
                },
                _ => {eprintln!("Could not find platform {platform}");break}
            };

            match send_webhook(&domain,diff.0, &endpoint).await {
                Ok(_r) => {println!("Webhook notification successfully sent!")},
                Err(e) => {eprintln!("Something went wrong while trying to send webhook: {e}")}
            };
        }
    }
}

async fn get_shodan_subdomains(
    domain: String,
    shodan_api_key: &String,
) -> Result<String, reqwest::Error> {
    match get(format!("{SHODANURL}{domain}?key={shodan_api_key}")).await {
        Ok(response) => match response.text().await {
            Ok(subdomains) => Ok(subdomains),
            Err(e) => {
                eprintln!("Failed to read shodan response: {e}");
                return Err(e);
            }
        },
        Err(e) => {
            eprint!("Failed to query shodan for subdomains: {e}");
            return Err(e);
        }
    }
}

fn diff_subdomains(
    database_sd: &HashSet<String>,
    new_sd: &HashSet<String>,
) -> (HashSet<String>, HashSet<String>) {
    let added: HashSet<String> = new_sd
        .difference(&database_sd)
        .map(|s| (*s).clone())
        .collect();

    let removed: HashSet<String> = database_sd
        .difference(&new_sd)
        .map(|s| (*s).clone())
        .collect();

    (added, removed)
}

async fn send_webhook(domain: &str, new_subdomains: HashSet<String>, webhook_url: &str) -> Result<Response, reqwest::Error> {
    let mut content = String::new();
    content.push_str(&format!("New subdomains for {}:\n",domain));
    for domain in new_subdomains.iter() {
        content.push_str(&format!("+ {}\n", domain));
    }

    let mut json = HashMap::new();
    json.insert("text",content);
    Client::new().post(webhook_url).json(&json).send().await
}
