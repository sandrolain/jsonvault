use clap::{Arg, Command as ClapCommand};
use jsonvault::{Command, Response, TcpClient};
use serde_json::Value;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), String> {
    let matches = ClapCommand::new("jsonvault-client")
        .version("0.1.0")
        .about("Client for JsonVault - JSON key-value database")
        .arg(
            Arg::new("server")
                .short('s')
                .long("server")
                .value_name("ADDRESS")
                .help("Server address")
                .default_value("127.0.0.1:8080"),
        )
        .subcommand(ClapCommand::new("interactive").about("Interactive mode"))
        .subcommand(
            ClapCommand::new("set")
                .about("Set a value")
                .arg(Arg::new("key").required(true))
                .arg(Arg::new("value").required(true)),
        )
        .subcommand(
            ClapCommand::new("get")
                .about("Get a value")
                .arg(Arg::new("key").required(true)),
        )
        .subcommand(
            ClapCommand::new("delete")
                .about("Delete a value")
                .arg(Arg::new("key").required(true)),
        )
        .subcommand(
            ClapCommand::new("qget")
                .about("Execute a JSONPath query")
                .arg(Arg::new("key").required(true))
                .arg(Arg::new("query").required(true)),
        )
        .subcommand(
            ClapCommand::new("qset")
                .about("Set a sub-property using JSONPath")
                .arg(Arg::new("key").required(true))
                .arg(Arg::new("path").required(true))
                .arg(Arg::new("value").required(true)),
        )
        .subcommand(
            ClapCommand::new("merge")
                .about("Merge a value")
                .arg(Arg::new("key").required(true))
                .arg(Arg::new("value").required(true)),
        )
        .subcommand(ClapCommand::new("ping").about("Ping the server"))
        .get_matches();

    let server_address = matches.get_one::<String>("server").unwrap();

    if matches.subcommand_matches("interactive").is_some() {
        run_interactive_mode(server_address).await?;
    } else {
        run_single_command(&matches, server_address).await?;
    }

    Ok(())
}

async fn run_single_command(
    matches: &clap::ArgMatches,
    server_address: &str,
) -> Result<(), String> {
    let mut client = TcpClient::connect(server_address).await?;

    let command = match matches.subcommand() {
        Some(("set", sub_matches)) => {
            let key = sub_matches.get_one::<String>("key").unwrap().clone();
            let value_str = sub_matches.get_one::<String>("value").unwrap();
            let value: Value = serde_json::from_str(value_str)
                .map_err(|e| format!("Invalid JSON value: {}", e))?;
            Command::Set { key, value }
        }
        Some(("get", sub_matches)) => {
            let key = sub_matches.get_one::<String>("key").unwrap().clone();
            Command::Get { key }
        }
        Some(("delete", sub_matches)) => {
            let key = sub_matches.get_one::<String>("key").unwrap().clone();
            Command::Delete { key }
        }
        Some(("qget", sub_matches)) => {
            let key = sub_matches.get_one::<String>("key").unwrap().clone();
            let query = sub_matches.get_one::<String>("query").unwrap().clone();
            Command::QGet { key, query }
        }
        Some(("qset", sub_matches)) => {
            let key = sub_matches.get_one::<String>("key").unwrap().clone();
            let path = sub_matches.get_one::<String>("path").unwrap().clone();
            let value_str = sub_matches.get_one::<String>("value").unwrap();
            let value: Value = serde_json::from_str(value_str)
                .map_err(|e| format!("Invalid JSON value: {}", e))?;
            Command::QSet { key, path, value }
        }
        Some(("merge", sub_matches)) => {
            let key = sub_matches.get_one::<String>("key").unwrap().clone();
            let value_str = sub_matches.get_one::<String>("value").unwrap();
            let value: Value = serde_json::from_str(value_str)
                .map_err(|e| format!("Invalid JSON value: {}", e))?;
            Command::Merge { key, value }
        }
        Some(("ping", _)) => Command::Ping,
        _ => {
            eprintln!("No command specified. Use --help to see available commands.");
            std::process::exit(1);
        }
    };

    let response = client.send_command(command).await?;
    print_response(&response);

    client.close().await?;
    Ok(())
}

async fn run_interactive_mode(server_address: &str) -> Result<(), String> {
    println!("Interactive mode for JSON DB client");
    println!("Connected to: {}", server_address);
    println!("Available commands:");
    println!("  set <key> <json_value>    - Set a value");
    println!("  get <key>                 - Get a value");
    println!("  delete <key>              - Delete a value");
    println!("  qget <key> <query>        - Execute a JSONPath query");
    println!("  qset <key> <path> <value> - Set a sub-property using JSONPath");
    println!("  merge <key> <json_value>  - Merge a value");
    println!("  ping                      - Ping the server");
    println!("  quit/exit                 - Exit");
    println!();

    let mut client = TcpClient::connect(server_address).await?;

    loop {
        print!("json-db> ");
        io::stdout()
            .flush()
            .map_err(|e| format!("Flush error: {}", e))?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| format!("Read error: {}", e))?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input == "quit" || input == "exit" {
            break;
        }

        let parts: Vec<&str> = input.splitn(4, ' ').collect();

        let command = match parts[0] {
            "set" => {
                if parts.len() != 3 {
                    eprintln!("Usage: set <key> <json_value>");
                    continue;
                }
                let key = parts[1].to_string();
                match serde_json::from_str::<Value>(parts[2]) {
                    Ok(value) => Command::Set { key, value },
                    Err(e) => {
                        eprintln!("Invalid JSON value: {}", e);
                        continue;
                    }
                }
            }
            "get" => {
                if parts.len() != 2 {
                    eprintln!("Usage: get <key>");
                    continue;
                }
                Command::Get {
                    key: parts[1].to_string(),
                }
            }
            "delete" => {
                if parts.len() != 2 {
                    eprintln!("Usage: delete <key>");
                    continue;
                }
                Command::Delete {
                    key: parts[1].to_string(),
                }
            }
            "qget" => {
                if parts.len() != 3 {
                    eprintln!("Usage: qget <key> <query>");
                    continue;
                }
                Command::QGet {
                    key: parts[1].to_string(),
                    query: parts[2].to_string(),
                }
            }
            "qset" => {
                if parts.len() != 4 {
                    eprintln!("Usage: qset <key> <path> <json_value>");
                    continue;
                }
                let key = parts[1].to_string();
                let path = parts[2].to_string();
                match serde_json::from_str::<Value>(parts[3]) {
                    Ok(value) => Command::QSet { key, path, value },
                    Err(e) => {
                        eprintln!("Invalid JSON value: {}", e);
                        continue;
                    }
                }
            }
            "merge" => {
                if parts.len() != 3 {
                    eprintln!("Usage: merge <key> <json_value>");
                    continue;
                }
                let key = parts[1].to_string();
                match serde_json::from_str::<Value>(parts[2]) {
                    Ok(value) => Command::Merge { key, value },
                    Err(e) => {
                        eprintln!("Invalid JSON value: {}", e);
                        continue;
                    }
                }
            }
            "ping" => Command::Ping,
            _ => {
                eprintln!("Unknown command: {}", parts[0]);
                continue;
            }
        };

        match client.send_command(command).await {
            Ok(response) => print_response(&response),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    client.close().await?;
    println!("Disconnected from server.");
    Ok(())
}

fn print_response(response: &Response) {
    match response {
        Response::Ok(Some(value)) => {
            println!(
                "{}",
                serde_json::to_string_pretty::<Value>(value).unwrap_or_else(|_| value.to_string())
            );
        }
        Response::Ok(None) => {
            println!("OK");
        }
        Response::Error(msg) => {
            eprintln!("Error: {}", msg);
        }
        Response::Pong => {
            println!("PONG");
        }
    }
}
