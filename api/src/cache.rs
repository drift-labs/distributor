use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use anchor_lang::{AccountDeserialize, Discriminator, __private::base64};
use dashmap::DashMap;
use futures::future::join_all;
use futures_util::StreamExt;
use merkle_distributor::state::claim_status::ClaimStatus;
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

#[derive(Clone)]
pub struct DataAndSlot<T> {
    pub data: T,
    pub slot: u64,
}

impl Deref for Cache {
    type Target = DashMap<String, DataAndSlot<ClaimStatus>>;

    fn deref(&self) -> &Self::Target {
        &self.cache
    }
}

impl DerefMut for Cache {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::get_mut(&mut self.cache).expect("Attempt to mutate a shared Cache")
    }
}

pub struct Cache {
    /// map from claimant to ClaimStatus account;
    cache: Arc<DashMap<String, DataAndSlot<ClaimStatus>>>,
    // rpc_client: RpcClient,
    // pubsub_client: PubsubClient,
    program_id: Pubkey,

    pub subscribed: bool,
    unsubscriber: Option<tokio::sync::mpsc::Sender<()>>,
    distributors: Vec<SingleDistributor>,
}

impl Cache {
    pub fn new(program_id: Pubkey, distributors: Vec<SingleDistributor>) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            program_id,
            subscribed: false,
            unsubscriber: None,
            distributors,
        }
    }

    pub fn get(&self, claimant: &String) -> Option<DataAndSlot<ClaimStatus>> {
        self.cache.get(claimant).map(|r| r.value().clone())
    }

    pub fn _len(&self) -> usize {
        self.cache.len()
    }

    /// starts a tokio task in the background that handles updating the cache.
    /// It starts a ProgramSubscribe from solana rpc and then does a getProgramAccounts
    /// to update the cache.
    pub async fn subscribe(&mut self, rpc_url: String, ws_url: String) -> Result<(), ApiError> {
        let ws_url = ws_url.clone();
        let rpc_url = rpc_url.clone();
        let program_id = self.program_id.clone();

        let (unsub_tx, mut unsub_rx) = tokio::sync::mpsc::channel::<()>(1);
        self.unsubscriber = Some(unsub_tx);

        // channel to send rpc things from the background task to cache updating task
        // payload: (ClaimStatusPubkey, ClaimStatus)
        let (update_tx, mut update_rx) =
            tokio::sync::mpsc::channel::<(String, DataAndSlot<ClaimStatus>)>(1000);

        // Clone the sender to move into the async block
        let update_tx_clone = update_tx.clone();

        let gpa_config = RpcProgramAccountsConfig {
            filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new(
                0,
                solana_rpc_client_api::filter::MemcmpEncodedBytes::Bytes(
                    ClaimStatus::DISCRIMINATOR.into(),
                ),
            ))]),
            account_config: RpcAccountInfoConfig {
                commitment: Some(CommitmentConfig::confirmed()),
                encoding: Some(UiAccountEncoding::Base64),
                data_slice: None,
                min_context_slot: None,
            },
            with_context: Some(true),
        };
        let gpa_config_clone_1 = gpa_config.clone();

        println!("Cache background updater starting up");

        let mut attempt = 0;
        let mut unsubscribed = false;
        let max_reconnection_attempts = 20;
        let base_delay = tokio::time::Duration::from_secs(5);

        tokio::spawn({
            async move {
                loop {
                    let gpa_config_clone_2 = gpa_config.clone();
                    let pubsub_client = PubsubClient::new(&ws_url).await.unwrap();
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
                                _ = unsub_rx.recv() => {
                                    println!("Unsubscribing.");
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

        // TODO: do gpa for each merkle distributor, to avoid getting truncated by RPC.
        let rpc_client = RpcClient::new(rpc_url.clone());
        match rpc_client
            .get_program_accounts_with_config(&program_id, gpa_config_clone_1)
            .await
        {
            Ok(accounts) => {
                let start_time = std::time::Instant::now();
                let total_accounts = accounts.len();
                let tasks: Vec<_> = accounts
                    .into_iter()
                    .map(|(pubkey, account)| {
                        let update_tx_clone = update_tx.clone();
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

        let cache_clone = Arc::clone(&self.cache);
        tokio::spawn(async move {
            while let Some((pubkey, data)) = update_rx.recv().await {
                match cache_clone.get(&pubkey) {
                    Some(current) => {
                        if current.slot <= data.slot {
                            println!(
                                "Updating cache with newer slot for claimant: {} ({})",
                                data.data.claimant.to_string(),
                                pubkey
                            );
                            cache_clone.insert(data.data.claimant.to_string(), data);
                        } else {
                            println!(
                                "Got an update with older slot than what's in cache for claimant: {} ({})",
                                data.data.claimant.to_string(),
                                pubkey
                            );
                        }
                    }
                    None => {
                        println!(
                            "Inserting new data into cache for claimant: {} ({})",
                            data.data.claimant.to_string(),
                            pubkey
                        );
                        cache_clone.insert(data.data.claimant.to_string(), data);
                    }
                };
            }
        });

        Ok(())
    }
}
