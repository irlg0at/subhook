use std::fs;

fn main() {
    let shodan_api_key = fs::read_to_string("/run/secrets/shodan_api_key").expect("Not able to read Shodan secret");
    println!("{shodan_api_key}");
}
