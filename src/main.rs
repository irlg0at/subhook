use reqwest::{get,Client, RequestBuilder, Response};
use clokwerk::{AsyncScheduler, TimeUnits, Job};
use rusqlite::Connection;
use std::collections::{HashSet,HashMap};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;

mod db;
mod domains;
use domains::Subdomains;

const SHODANURL: &str = "https://api.shodan.io/dns/domain/";
const SLACK_ENDPOINT: &str = "aaaaah";
const DISCORD_ENDPOINT: &str = "aaaaah";


#[tokio::main]
async fn main() {
    let time = "08:00";
    let mut scheduler = AsyncScheduler::with_tz(chrono::Utc);



    scheduler.every(1.day()).at(time).run(|| 
        async {
            let shodan_api_key: String =
                fs::read_to_string("/run/secrets/shodan_api_key").expect("Not able to read Shodan secret");
            let domains =
                BufReader::new(File::open("/domains.txt").expect("Not able to read domains.txt file."));
            let db_path = Path::new("/data/domain.sqlite");
            if !db_path.exists() {
                db::initialize_db(db_path).expect("Could not create or reach database.");
                println!("Database created!")
            }

            println!("Running update of database...");
            update(domains, shodan_api_key, db_path).await;
            println!("Update done!");

    });

    loop {
        scheduler.run_pending().await;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

async fn update(domains: BufReader<File>, shodan_api_key: String, db_path: &Path) -> () {
    let webhook_url = "url";
    let platform = "discord";


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

        if !diff.0.is_empty() { 
            let endpoint = match platform {
                "slack" => {
                    format!("{}{}",webhook_url,SLACK_ENDPOINT)
                },

                "discord" => {
                    format!("{}{}",webhook_url,DISCORD_ENDPOINT)
                },
                _ => {eprintln!("Could not find platform {platform}");break}
            };

            match send_webhook(diff.0, &endpoint).await {
                Ok(_r) => {println!("Webhook notification successfully sent!")},
                Err(e) => {eprint!("Something went wrong while trying to send webhook: {e}")}
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

async fn send_webhook(new_domains: HashSet<String>, webhook_url: &str) -> Result<Response, reqwest::Error> {
    let mut content = HashMap::new();
    content.insert("text","test");
    Client::new().post(webhook_url).json(&content).send().await
}
