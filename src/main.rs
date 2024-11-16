use std::fs::{self, File};

use reqwest::get;

const DNSURL: &str = "https://api.shodan.io/dns/domain/";


fn main() {
    let shodan_api_key:String = fs::read_to_string("/run/secrets/shodan_api_key").expect("Not able to read Shodan secret");
    let domains = File::open("/domains.txt").expect("Not able to read domains.txt file."); 

    println!("{subdomains}");
}

async fn get_subdomains(domain:String) {
    let ubdomains = get("https://www.rust-lang.org")
    return subdomains;
} 

