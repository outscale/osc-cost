pub mod human;
pub mod markdown;
pub mod json;
pub mod ods;
pub mod prometheus;

fn get_currency(region: &str) -> String {
    match region {
        "eu-west-2" | "cloudgouv-eu-west-1" => String::from("€"),
        "ap-northeast-1" => String::from("¥"),
        "us-east-2" | "us-west-1" => String::from("$"),
        _ => String::from("€"),
    }
}
