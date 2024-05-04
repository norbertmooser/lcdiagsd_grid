use std::{fs::File, io::{BufRead, BufReader}};
use redis::{Client, Commands};
use serde::{Deserialize, Serialize};
use serde_json;
use std::process;
use prettytable::{Table, Row, Cell, format};
use chrono::{NaiveDateTime, Utc, TimeZone};
use std::net::Ipv4Addr;


// Define a struct to represent lease information
#[derive(Debug, Serialize, Deserialize, Clone)]
struct LeaseInfo {
    timestamp: u64,
    mac_address: String,
    ip_address: String,
    hostname: String,
}

fn main() {
    // Read dnsmasq.leases file
    let filename = "dnsmasq.leases";
    let leases: Vec<LeaseInfo> = read_leases_file(filename).expect("Error reading leases file");

    // Convert leases to JSON
    let json_data: String = serde_json::to_string(&leases).expect("Error serializing to JSON");

    // Write JSON data to Redis
    let redis_url: &str = "redis://127.0.0.1/";
    let redis_key: &str = "dnsmasq:leases";
    write_to_redis(redis_url, redis_key, &json_data).expect("Error writing to Redis");

    // Read JSON data from Redis
    let json_data = read_from_redis(redis_url, redis_key).expect("Error reading from Redis");

    // Parse JSON data into Vec<LeaseInfo>
    let leases: Vec<LeaseInfo> = serde_json::from_str(&json_data).expect("Error parsing JSON");

    // Display lease information in a table
    display_table(&leases);
}

fn display_table(leases: &[LeaseInfo]) {
    let mut sorted_leases = leases.to_vec();
    sorted_leases.sort_by(|a, b| {
        // Parse IP addresses and compare them
        let ip_a: Ipv4Addr = a.ip_address.parse::<Ipv4Addr>().unwrap();
        let ip_b: Ipv4Addr = b.ip_address.parse::<Ipv4Addr>().unwrap();
        ip_a.cmp(&ip_b)
    });

    let mut table = Table::new();

    let format: format::TableFormat = format::FormatBuilder::new()
        .column_separator('|')
        .borders('|')
        .separators(
            &[format::LinePosition::Top, format::LinePosition::Bottom],
            format::LineSeparator::new('-', '+', '+', '+'),
        )
        .separators(
            &[format::LinePosition::Title],
            format::LineSeparator::new('-', '+', '+', '+'),
        )
        .padding(1, 1)
        .build();
    table.set_format(format);

    // Define table headers
    table.add_row(Row::new(vec![
        Cell::new("Timestamp"),
        Cell::new("MAC Address"),
        Cell::new("IP Address"),
        Cell::new("Hostname"),
    ]));

    // Define separator
    table.add_row(Row::new(vec![
        Cell::new("--"),
        Cell::new("--"),
        Cell::new("--"),
        Cell::new("--"),
    ]));

    // Populate table rows with sorted lease information
    for lease in sorted_leases {
        // Convert timestamp to YYMMDD_hhmm format
        let datetime = NaiveDateTime::from_timestamp_opt(lease.timestamp as i64, 0)
            .expect("Invalid timestamp");
        let utc_datetime = Utc.from_utc_datetime(&datetime);
        let formatted_timestamp = utc_datetime.format("%y%m%d_%H%M").to_string();

        table.add_row(Row::new(vec![
            Cell::new(&formatted_timestamp),
            Cell::new(&lease.mac_address),
            Cell::new(&lease.ip_address),
            Cell::new(&lease.hostname),
        ]));
    }

    // Print the table
    table.printstd();
}


fn read_from_redis(redis_url: &str, redis_key: &str) -> redis::RedisResult<String> {
    let client: Client = Client::open(redis_url)?;
    let mut con: redis::Connection = client.get_connection()?;
    let data: String = con.get(redis_key)?;
    Ok(data)
}


fn read_leases_file(filename: &str) -> std::io::Result<Vec<LeaseInfo>> {
    let file = match File::open(filename) {
        Ok(file) => file,
        Err(_) => {
            eprintln!("Error reading leases file: File not found: {}", filename);
            process::exit(1); // Exit with error code 1
        }
    };
    let reader: BufReader<File> = BufReader::new(file);
    let mut leases: Vec<LeaseInfo> = Vec::new();

    for line in reader.lines() {
        let line: String = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let timestamp: u64 = parts[0].parse().unwrap_or_default();
            let mac_address: String = parts[1].to_string();
            let ip_address: String = parts[2].to_string();
            let hostname: String
             = parts[3].to_string();
            leases.push(LeaseInfo { timestamp, mac_address, ip_address, hostname });
        }
    }

    Ok(leases)
}

fn write_to_redis(redis_url: &str, redis_key: &str, data: &str) -> redis::RedisResult<()> {
    let client = Client::open(redis_url)?;
    let mut con = client.get_connection()?;
    let _: () = con.set(redis_key, data)?;
    Ok(())
}
