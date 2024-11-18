use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Data {
    tags: Option<Vec<String>>,
    subdomain: String,
    #[serde(rename = "type")]
    r#type: String,
    ports: Option<Vec<i32>>,
    value: String,
    last_seen: String,
}

#[derive(Deserialize, Debug)]
pub struct Subdomains {
    pub domain: String,
    pub subdomains: Vec<String>,
    pub data: Vec<Data>,
}
