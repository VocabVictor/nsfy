use crate::config::{self, StoredServer, StoredTopic};
use clap::{Args, Parser, Subcommand};
use serde_json::json;

#[path = "cli_http.rs"]
mod http;
use http::{client, env_token, print_json, print_response, request};

#[derive(Parser)]
#[command(name = "nsfy-cli", version, about = "Command-line client for nsfy")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage saved servers.
    Server {
        #[command(subcommand)]
        command: ServerCommand,
    },
    /// Manage topic subscriptions shared with the desktop GUI.
    Topic {
        #[command(subcommand)]
        command: TopicCommand,
    },
    /// Publish one message.
    Publish(PublishArgs),
    /// Fetch cached messages from a topic.
    Poll(TopicRequest),
    /// Query server statistics.
    Status(ServerRequest),
    /// Print the shared GUI/CLI config path.
    ConfigPath,
}

#[derive(Subcommand)]
enum ServerCommand {
    List,
    Add {
        #[arg(long)]
        url: String,
        #[arg(long)]
        name: String,
        /// Prefer NSFY_AUTH_TOKEN so the token is not visible in process lists.
        #[arg(long)]
        token: Option<String>,
    },
    Remove {
        /// Configured server name or URL.
        server: String,
    },
}

#[derive(Subcommand)]
enum TopicCommand {
    List,
    Add {
        #[arg(long)]
        server: String,
        #[arg(long)]
        topic: String,
    },
    Remove {
        #[arg(long)]
        server: String,
        #[arg(long)]
        topic: String,
    },
}

#[derive(Args)]
struct ServerRequest {
    /// Configured server name or URL. Defaults to the first saved server.
    #[arg(long)]
    server: Option<String>,
}

#[derive(Args)]
struct TopicRequest {
    /// Configured server name or URL. Defaults to the first saved server.
    #[arg(long)]
    server: Option<String>,
    #[arg(long)]
    topic: String,
    #[arg(long)]
    since: Option<String>,
}

#[derive(Args)]
struct PublishArgs {
    /// Configured server name or URL. Defaults to the first saved server.
    #[arg(long)]
    server: Option<String>,
    #[arg(long)]
    topic: String,
    #[arg(long, default_value = "")]
    title: String,
    #[arg(long)]
    message: String,
    #[arg(long, default_value_t = 3, value_parser = clap::value_parser!(u8).range(1..=5))]
    priority: u8,
    #[arg(long = "tag")]
    tags: Vec<String>,
    /// Slash-separated category path, for example work/agents/codex.
    #[arg(long)]
    category: Option<String>,
}

pub fn run() -> Result<(), String> {
    match Cli::parse().command {
        Command::Server { command } => run_server(command),
        Command::Topic { command } => run_topic(command),
        Command::Publish(args) => publish(args),
        Command::Poll(args) => poll(args),
        Command::Status(args) => status(args),
        Command::ConfigPath => {
            println!("{}", config::config_path()?.display());
            Ok(())
        }
    }
}

fn run_server(command: ServerCommand) -> Result<(), String> {
    let mut stored = config::load()?;
    match command {
        ServerCommand::List => {
            let output: Vec<_> = stored
                .servers
                .iter()
                .map(|server| {
                    json!({
                        "name": server.name,
                        "url": server.url,
                        "tokenConfigured": server.token.is_some(),
                    })
                })
                .collect();
            print_json(&output)
        }
        ServerCommand::Add { url, name, token } => {
            let url = config::normalize_url(&url)?;
            let name = name.trim();
            if name.is_empty() {
                return Err("server name cannot be empty".into());
            }
            let token = token
                .or_else(env_token)
                .filter(|value| !value.trim().is_empty());
            if let Some(server) = stored.servers.iter_mut().find(|item| item.url == url) {
                server.name = name.into();
                server.token = token;
            } else {
                stored.servers.push(StoredServer {
                    url: url.clone(),
                    name: name.into(),
                    token,
                });
            }
            config::save(&stored)?;
            print_json(&json!({ "saved": true, "name": name, "url": url }))
        }
        ServerCommand::Remove { server } => {
            let resolved = resolve_server(&stored.servers, Some(&server))?;
            let url = resolved.url;
            stored.servers.retain(|item| item.url != url);
            stored.topics.retain(|topic| topic.server != url);
            config::save(&stored)?;
            print_json(&json!({ "removed": true, "url": url }))
        }
    }
}

fn run_topic(command: TopicCommand) -> Result<(), String> {
    let mut stored = config::load()?;
    match command {
        TopicCommand::List => print_json(&stored.topics),
        TopicCommand::Add { server, topic } => {
            let server = resolve_server(&stored.servers, Some(&server))?.url;
            let topic = config::validate_topic(&topic)?;
            if !stored
                .topics
                .iter()
                .any(|item| item.server == server && item.name == topic)
            {
                stored.topics.push(StoredTopic {
                    name: topic.clone(),
                    server: server.clone(),
                    unread: 0,
                });
                config::save(&stored)?;
            }
            print_json(&json!({ "subscribed": true, "server": server, "topic": topic }))
        }
        TopicCommand::Remove { server, topic } => {
            let server = resolve_server(&stored.servers, Some(&server))?.url;
            let topic = config::validate_topic(&topic)?;
            stored
                .topics
                .retain(|item| item.server != server || item.name != topic);
            config::save(&stored)?;
            print_json(&json!({ "subscribed": false, "server": server, "topic": topic }))
        }
    }
}

fn publish(args: PublishArgs) -> Result<(), String> {
    if args.message.trim().is_empty() {
        return Err("message cannot be empty".into());
    }
    let stored = config::load()?;
    let server = resolve_server(&stored.servers, args.server.as_deref())?;
    let topic = config::validate_topic(&args.topic)?;
    let category = config::parse_category(args.category.as_deref())?;
    let response = request(client()?.post(format!("{}/{topic}", server.url)), &server)
        .json(&json!({
            "title": args.title,
            "message": args.message,
            "priority": args.priority,
            "tags": args.tags,
            "category": category,
        }))
        .send()
        .map_err(|e| e.to_string())?;
    print_response(response)
}

fn poll(args: TopicRequest) -> Result<(), String> {
    let stored = config::load()?;
    let server = resolve_server(&stored.servers, args.server.as_deref())?;
    let topic = config::validate_topic(&args.topic)?;
    let mut url = format!("{}/{topic}/json", server.url);
    if let Some(since) = args.since {
        url.push_str("?since=");
        url.push_str(&since);
    }
    let response = request(client()?.get(url), &server)
        .send()
        .map_err(|e| e.to_string())?;
    print_response(response)
}

fn status(args: ServerRequest) -> Result<(), String> {
    let stored = config::load()?;
    let server = resolve_server(&stored.servers, args.server.as_deref())?;
    let response = request(client()?.get(format!("{}/", server.url)), &server)
        .send()
        .map_err(|e| e.to_string())?;
    print_response(response)
}

fn resolve_server(
    servers: &[StoredServer],
    selector: Option<&str>,
) -> Result<StoredServer, String> {
    match selector {
        Some(value) => {
            if let Some(server) = servers
                .iter()
                .find(|server| server.url == value.trim_end_matches('/') || server.name == value)
            {
                let mut server = server.clone();
                server.url = config::normalize_url(&server.url)?;
                return Ok(server);
            }
            let url = config::normalize_url(value)?;
            Ok(StoredServer {
                name: url.clone(),
                url,
                token: None,
            })
        }
        None => {
            let mut server = servers
                .first()
                .cloned()
                .ok_or_else(|| "no server is configured".to_string())?;
            server.url = config::normalize_url(&server.url)?;
            Ok(server)
        }
    }
}
