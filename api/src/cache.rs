use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use anchor_lang::{AccountDeserialize, Discriminator, __private::base64};
use dashmap::DashMap;
use futures::future::join_all;
use futures_util::StreamExt;
use merkle_distributor::state::{claim_status::ClaimStatus, merkle_distributor::MerkleDistributor};
use solana_account_decoder::{UiAccountData, UiAccountEncoding};
use solana_program::pubkey::Pubkey;
use solana_pubsub_client::nonblocking::pubsub_client::PubsubClient;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::{
    config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    filter::{Memcmp, RpcFilterType},
};
use solana_sdk::commitment_config::CommitmentConfig;

use crate::{error::ApiError, router::SingleDistributor};

const INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct DataAndSlot<T> {
    pub data: T,
    pub slot: u64,
}

impl Deref for Cache {
    type Target = DashMap<String, DataAndSlot<ClaimStatus>>;

    fn deref(&self) -> &Self::Target {
        &self.claim_status_cache
    }
}

impl DerefMut for Cache {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::get_mut(&mut self.claim_status_cache).expect("Attempt to mutate a shared Cache")
    }
}

#[derive(Clone)]
pub struct Cache {
    /// map from claimant to ClaimStatus account;
    claim_status_cache: Arc<DashMap<String, DataAndSlot<ClaimStatus>>>,
    /// map from pubkey to MerkleDistributor account;
    distributor_cache: Arc<DashMap<String, MerkleDistributor>>,
    // rpc_client: RpcClient,
    // pubsub_client: PubsubClient,
    program_id: Pubkey,

    pub subscribed: bool,
    // unsubscriber: Option<tokio::sync::watch::Sender<()>>,
    distributors: Vec<SingleDistributor>,
    unvested_users: Option<HashMap<String, u128>>,

    pub default_start_ts: i64,
    pub default_end_ts: i64,
    pub default_mint: String,
}

impl Cache {
    pub fn new(
        program_id: Pubkey,
        distributors: Vec<SingleDistributor>,
        unvested_users: Option<HashMap<String, u128>>,
        default_start_ts: i64,
        default_end_ts: i64,
        default_mint: String,
    ) -> Self {
        Self {
            claim_status_cache: Arc::new(DashMap::new()),
            distributor_cache: Arc::new(DashMap::new()),
            program_id,
            subscribed: false,
            // unsubscriber: None,
            distributors,
            unvested_users,
            default_start_ts,
            default_end_ts,
            default_mint,
        }
    }

    pub fn get_claim_status(&self, claimant: &String) -> Option<DataAndSlot<ClaimStatus>> {
        self.claim_status_cache
            .get(claimant)
            .map(|r| r.value().clone())
    }

    pub fn get_all_distributors(&self) -> Vec<(String, MerkleDistributor)> {
        self.distributor_cache
            .iter()
            .map(|r| (r.key().clone(), r.value().clone()))
            .collect::<Vec<(String, MerkleDistributor)>>()
    }

    pub fn get_distributor(&self, pubkey: &String) -> Option<MerkleDistributor> {
        self.distributor_cache
            .get(pubkey)
            .map(|r| r.value().clone())
    }

    pub fn _len(&self) -> usize {
        self.claim_status_cache.len()
    }

    async fn hydrate_cache_for_config(
        &mut self,
        gpa_config: RpcProgramAccountsConfig,
        rpc_client: &RpcClient,
        ws_url: String,
        update_tx: tokio::sync::mpsc::Sender<(String, DataAndSlot<ClaimStatus>)>,
        mut unsub_rx: tokio::sync::watch::Receiver<()>,
    ) -> Result<(), ApiError> {
        let mut attempt = 0;
        let mut unsubscribed = false;
        let max_reconnection_attempts = 20;
        let base_delay = tokio::time::Duration::from_secs(5);
        let program_id = self.program_id.clone();

        let gpa_config_clone = gpa_config.clone();
        let update_tx_clone = update_tx.clone();
        tokio::spawn({
            async move {
                loop {
                    let gpa_config_clone_2 = gpa_config_clone.clone();
                    let pubsub_client = PubsubClient::new(ws_url.as_str()).await.unwrap();
                    match pubsub_client
                        .program_subscribe(&program_id, Some(gpa_config_clone_2))
                        .await
                    {
                        Ok((mut accounts, unsubscriber)) => loop {
                            tokio::select! {
                                message = accounts.next() => {
                                    attempt = 0;
                                    match message {
                                        Some(message) => {
                                            let data_str = match message.value.account.data {
                                                UiAccountData::Binary(encoded, _) => encoded,
                                                _ => {
                                                    println!("Unsupported account data");
                                                    continue;
                                                }
                                            };
                                            let decoded_data = match base64::decode(data_str.as_bytes()) {
                                                Ok(data) => data,
                                                Err(_) => {
                                                    println!("Failed to decode base64 data");
                                                    continue;
                                                }
                                            };
                                            let data = match ClaimStatus::try_deserialize(&mut decoded_data.as_slice()).map_err(|err| ApiError::InternalError(Box::new(err))) {
                                                Ok(data) => data,
                                                Err(_) => {
                                                    println!("Failed to deserialize ClaimStatus");
                                                    continue;
                                                }
                                            };
                                            if let Err(_) = update_tx_clone.send((
                                                message.value.pubkey,
                                                DataAndSlot{
                                                    data,
                                                    slot: message.context.slot,
                                                })
                                            ).await {
                                                println!("Failed to send update through channel");
                                                continue;
                                            }
                                        }
                                        None => {
                                            println!("cach.subscribe stream ended");
                                            unsubscriber().await;
                                            break;
                                        }
                                    }
                                }
                                _ = unsub_rx.changed() => {
                                    println!("Cache update loop unsubscribing.");
                                    unsubscriber().await;
                                    unsubscribed = true;
                                    break;
                                }
                            }
                        },
                        Err(e) => {
                            println!("Error in program_subscribe: {:?}", e);
                            attempt += 1;
                            if attempt >= max_reconnection_attempts {
                                println!("Max websocket reconnection attempts reached.");
                                return Err(ApiError::MaxReconnectionAttemptsReached);
                            }
                        }
                    };
                    if unsubscribed {
                        println!("Cache background loop ended.");
                        return Ok::<(), ApiError>(());
                    }
                    if attempt >= max_reconnection_attempts {
                        println!("Max websocket reconnection attempts reached.");
                        return Err(ApiError::MaxReconnectionAttemptsReached);
                    }

                    let delay_duration = base_delay * 2_u32.pow(attempt);
                    println!("Websocket Reconnecting in {:?}", delay_duration);
                    tokio::time::sleep(delay_duration).await;
                    attempt += 1;
                }
            }
        });

        let update_tx_clone = update_tx.clone();
        match rpc_client
            .get_program_accounts_with_config(&program_id, gpa_config)
            .await
        {
            Ok(accounts) => {
                let start_time = std::time::Instant::now();
                let total_accounts = accounts.len();
                let tasks: Vec<_> = accounts
                    .into_iter()
                    .map(|(pubkey, account)| {
                        let update_tx_clone = update_tx_clone.clone();
                        tokio::spawn(async move {
                            let data = ClaimStatus::try_deserialize(&mut account.data.as_slice())
                                .map_err(|err| ApiError::InternalError(Box::new(err)))
                                .unwrap();
                            update_tx_clone
                                .send((pubkey.to_string(), DataAndSlot { data, slot: 0 }))
                                .await
                                .unwrap();
                        })
                    })
                    .collect();

                join_all(tasks).await;
                let total_duration = start_time.elapsed();
                println!(
                    "Total deserialization time for {} accounts: {:?}",
                    total_accounts, total_duration
                );
            }
            Err(e) => {
                println!("Error in gpa_resp: {:?}", e);
            }
        };

        Ok(())
    }

    fn get_distributor_keys(&self) -> Vec<Pubkey> {
        self.distributors
            .iter()
            .map(|d| Pubkey::from_str(&d.distributor_pubkey).unwrap())
            .collect::<Vec<Pubkey>>()
    }

    pub fn get_unvested_amount(&self, user_pubkey: String) -> u128 {
        self.unvested_users
            .as_ref()
            .map_or(0, |m| *(m.get(&user_pubkey).unwrap_or(&0)))
    }

    async fn hydrate_distributors_cache(self: Arc<Self>, rpc_client: &RpcClient) {
        dbg!("Hydrating distributor cache");
        // hydrate distributor cache
        let distributor_cache = Arc::clone(&self.distributor_cache);
        let distributors_to_load = self.distributors.clone();
        let distributor_keys_vec = self.get_distributor_keys();
        let distributor_keys = distributor_keys_vec.as_slice();
        match rpc_client
            .get_multiple_accounts_with_commitment(&distributor_keys, CommitmentConfig::confirmed())
            .await
        {
            Ok(accounts) => {
                for (index, account) in accounts.value.into_iter().enumerate() {
                    let distributor = distributors_to_load.get(index).unwrap();
                    match account {
                        Some(account) => {
                            let distributor_data =
                                MerkleDistributor::try_deserialize(&mut account.data.as_slice())
                                    .map_err(|err| ApiError::InternalError(Box::new(err)))
                                    .unwrap();
                            distributor_cache
                                .insert(distributor.distributor_pubkey.clone(), distributor_data);
                        }
                        None => {
                            println!(
                                "Error in gma for distributor: {}",
                                distributor.distributor_pubkey
                            );
                        }
                    }
                }
            }
            Err(e) => {
                println!("Error in distributor gma: {:?}", e);
            }
        }
    }

    /// Starts a tokio task in the background that handles updating the cache. getProgramAccounts
    /// and programSubscribe tasks should send account updates through to update_rx
    fn start_cache_updater(
        &mut self,
        mut update_rx: tokio::sync::mpsc::Receiver<(String, DataAndSlot<ClaimStatus>)>,
    ) {
        let cache_clone = Arc::clone(&self.claim_status_cache);
        tokio::spawn(async move {
            while let Some((pubkey, data)) = update_rx.recv().await {
                match cache_clone.get(&pubkey) {
                    Some(current) => {
                        if current.slot <= data.slot {
                            println!(
                                "Updating cache with newer slot for claimant: {} ({}) on {} tree",
                                data.data.claimant.to_string(),
                                pubkey,
                                data.data.distributor.to_string()
                            );
                            cache_clone.insert(data.data.claimant.to_string(), data);
                        } else {
                            println!(
                                "Got an update with older slot than what's in cache for claimant: {} ({}) on {} tree",
                                data.data.claimant.to_string(),
                                pubkey,
                                data.data.distributor.to_string()
                            );
                        }
                    }
                    None => {
                        println!(
                            "Inserting new data into cache for claimant: {} ({}) on {} tree",
                            data.data.claimant.to_string(),
                            pubkey,
                            data.data.distributor.to_string()
                        );
                        cache_clone.insert(data.data.claimant.to_string(), data);
                    }
                };
            }
        });
    }

    /// starts a tokio task in the background that handles updating the cache.
    /// It starts a ProgramSubscribe from solana rpc and then does a getProgramAccounts
    /// to update the cache.
    pub async fn subscribe(&mut self, rpc_url: String, ws_url: String) -> Result<(), ApiError> {
        let ws_url = ws_url.clone();
        let rpc_url = rpc_url.clone();

        let (unsub_tx, unsub_rx) = tokio::sync::watch::channel(());
        // self.unsubscriber = Some(unsub_tx);

        // channel to send rpc things from the background task to cache updating task
        // payload: (ClaimStatusPubkey, ClaimStatus)
        let (update_tx, update_rx) =
            tokio::sync::mpsc::channel::<(String, DataAndSlot<ClaimStatus>)>(1000);

        self.start_cache_updater(update_rx);

        let rpc_client = RpcClient::new(rpc_url.clone());
        let distributor_keys_vec = self.get_distributor_keys();

        for distributor in distributor_keys_vec {
            let gpa_config = RpcProgramAccountsConfig {
                filters: Some(vec![
                    RpcFilterType::Memcmp(Memcmp::new(
                        0,
                        solana_rpc_client_api::filter::MemcmpEncodedBytes::Bytes(
                            ClaimStatus::DISCRIMINATOR.into(),
                        ),
                    )),
                    RpcFilterType::Memcmp(Memcmp::new(
                        73,
                        solana_rpc_client_api::filter::MemcmpEncodedBytes::Bytes(
                            distributor.to_bytes().into(),
                        ),
                    )),
                ]),
                account_config: RpcAccountInfoConfig {
                    commitment: Some(CommitmentConfig::confirmed()),
                    encoding: Some(UiAccountEncoding::Base64),
                    data_slice: None,
                    min_context_slot: None,
                },
                with_context: Some(true),
            };

            println!("Starting up background updater for {}", distributor);
            let update_tx = update_tx.clone();
            let unsub_rx = unsub_rx.clone();
            self.hydrate_cache_for_config(
                gpa_config,
                &rpc_client,
                ws_url.clone(),
                update_tx,
                unsub_rx,
            )
            .await?;
        }

        let arc_self = Arc::new(self.clone());
        let mut interval = tokio::time::interval(INTERVAL);

        tokio::task::spawn(async move {
            loop {
                arc_self
                    .clone()
                    .hydrate_distributors_cache(&rpc_client)
                    .await;
                interval.tick().await;
            }
        });

        Ok(())
    }
}
