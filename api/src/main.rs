mod cache;
mod error;
mod router;

use std::{
    collections::HashMap, fmt::Debug, fs, net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc,
};

use clap::Parser;
use csv::Reader;
use futures::future::join_all;
use jito_merkle_tree::{airdrop_merkle_tree::AirdropMerkleTree, utils::get_merkle_distributor_pda};
use router::RouterState;
use solana_program::pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use tokio::sync::Mutex;
use tracing::{info, instrument};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::{cache::Cache, error::ApiError, router::SingleDistributor};
pub type Result<T> = std::result::Result<T, ApiError>;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Bind address for the server
    #[clap(long, env, default_value_t = SocketAddr::from_str("0.0.0.0:7001").unwrap())]
    bind_addr: SocketAddr,

    /// Path of merkle tree
    #[clap(long, env)]
    merkle_tree_path: PathBuf,

    /// RPC url
    #[clap(long, env)]
    rpc_url: String,

    #[clap(long, env)]
    ws_url: String,

    /// Mint address of token in question
    #[clap(long, env)]
    mint: Pubkey,

    /// Program ID
    #[clap(long, env)]
    program_id: Pubkey,

    /// Path to csv file of accounts with vested tokens
    #[clap(long, env)]
    vested_accounts_path: Option<PathBuf>,
}

#[tokio::main]
#[instrument]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    tracing_subscriber::registry()
        .with(
            fmt::Layer::new()
                .with_writer(std::io::stdout)
                .event_format(fmt::format().with_line_number(true).with_file(true)),
        )
        .with(EnvFilter::from_default_env())
        .init();

    println!("args: {:?}", args);

    println!("starting server at {}", args.bind_addr);

    let rpc_client = RpcClient::new(args.rpc_url.clone());
    info!("started rpc client at {}", args.rpc_url);

    let mut paths: Vec<_> = fs::read_dir(&args.merkle_tree_path)
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    paths.sort_by_key(|dir| dir.path());

    let tree = Arc::new(Mutex::new(HashMap::new()));
    let distributors = Arc::new(Mutex::new(vec![]));
    let start_all_trees = std::time::Instant::now();
    println!("checking {} files for merkle trees", paths.len());

    let tasks: Vec<_> = paths
        .into_iter()
        .map(|file| {
            let distributors_clone = distributors.clone();
            let tree_clone = tree.clone();
            tokio::spawn(async move {
                let start_single_tree = std::time::Instant::now();
                let single_tree_path = file.path();
                if single_tree_path.is_dir() {
                    return;
                }
                if single_tree_path.extension().unwrap() != "json" {
                    println!("skipping {}", single_tree_path.display());
                    return;
                }
                let single_tree = match AirdropMerkleTree::new_from_file(&single_tree_path) {
                    Ok(tree) => tree,
                    Err(e) => {
                        eprintln!(
                            "error loading tree from {}: {}",
                            single_tree_path.display(),
                            e
                        );
                        return;
                    }
                };

                let (distributor_pubkey, _bump) = get_merkle_distributor_pda(
                    &args.program_id,
                    &args.mint,
                    single_tree.airdrop_version,
                );

                let mut distributors = distributors_clone.lock().await;
                distributors.push(SingleDistributor {
                    distributor_pubkey: distributor_pubkey.to_string(),
                    airdrop_version: single_tree.airdrop_version,
                    max_num_nodes: single_tree.max_num_nodes,
                    max_total_claim: single_tree.max_total_claim,
                });
                for node in single_tree.tree_nodes.iter() {
                    tree_clone
                        .lock()
                        .await
                        .insert(node.claimant, (distributor_pubkey, node.clone()));
                }
                let duration_single_tree = start_single_tree.elapsed();
                println!(
                    "loaded tree {} from {} with {} nodes in {:?}",
                    single_tree.airdrop_version,
                    single_tree_path.display(),
                    single_tree.tree_nodes.len(),
                    duration_single_tree
                );
            })
        })
        .collect();
    join_all(tasks).await;

    let duration_all_trees = start_all_trees.elapsed();
    println!("Done loading all trees in {:?}", duration_all_trees);

    let start_vested_users = std::time::Instant::now();
    let mut vested_users: Option<HashMap<String, u128>> = None;
    if let Some(vested_accounts_path) = args.vested_accounts_path {
        let vested_users_file = fs::File::open(vested_accounts_path)?;
        let vested_users_reader = std::io::BufReader::new(vested_users_file);
        let mut vested_users_csv = Reader::from_reader(vested_users_reader);
        let mut vested_users_map = HashMap::new();
        for record in vested_users_csv.records() {
            let record = match record {
                Ok(record) => record,
                Err(e) => {
                    eprintln!("Error parsing vested user record: {}", e);
                    continue;
                }
            };
            let user_pubkey = &record[0].to_string();
            let vested_amount = &record[1].parse::<u128>().unwrap();
            vested_users_map
                .entry(user_pubkey.to_string())
                .and_modify(|e| *e += *vested_amount)
                .or_insert(*vested_amount);
        }
        vested_users = Some(vested_users_map);
    } else {
        println!("No vested accounts path provided");
    }
    let duration_vested_users = start_vested_users.elapsed();
    println!(
        "Loaded {} vested users in {:?}",
        vested_users.clone().map_or(0, |m| m.len()),
        duration_vested_users
    );

    let mut cache = Cache::new(
        args.program_id,
        distributors.lock().await.clone(),
        vested_users,
    );
    cache.subscribe(args.rpc_url, args.ws_url).await?;

    let state = Arc::new(RouterState {
        tree: tree.lock().await.clone(),
        program_id: args.program_id,
        rpc_client,
        cache,
    });

    let app = router::get_routes(state);

    axum::Server::bind(&args.bind_addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;

    info!("done");
    Ok(())
}
