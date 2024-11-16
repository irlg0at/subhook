use std::fs::{self, File};
use std::io::{BufReader, BufRead};
use reqwest::get;
use tokio;

const SHODANURL: &str = "https://api.shodan.io/dns/domain/";

#[tokio::main]
async fn main() {
    let shodan_api_key:String = fs::read_to_string("/run/secrets/shodan_api_key").expect("Not able to read Shodan secret");
    let domains = BufReader::new(File::open("/domains.txt").expect("Not able to read domains.txt file.")); 

    for domain in domains.lines() {
        let subdomains = get_shodan_subdomains(domain.unwrap(), &shodan_api_key).await.unwrap();
        println!("{subdomains}");
    }
}

async fn get_shodan_subdomains(domain:String, shodan_api_key:&String) -> Option<String> {
    match get(format!("{SHODANURL}{domain}?key={shodan_api_key}")).await {
        Ok(response) => match response.text().await {
            Ok(subdomains) => Some(subdomains),
            Err(e) => {
                eprintln!("Failed to read shodan response: {e}");
                return None
            }
        },
        Err(e) =>  {
            eprint!("Failed to query shodan for subdomains: {e}");
            return None
        }
    }
} 

