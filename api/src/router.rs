use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Body,
    error_handling::HandleErrorLayer,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use http::Request;
use jito_merkle_tree::{airdrop_merkle_tree::UserProof, tree_node::TreeNode};
use merkle_distributor::state::merkle_distributor::MerkleDistributor;
use serde_derive::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use tower::{
    buffer::BufferLayer, limit::RateLimitLayer, load_shed::LoadShedLayer, timeout::TimeoutLayer,
    ServiceBuilder,
};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultOnResponse, TraceLayer},
    validate_request::ValidateRequestHeaderLayer,
    LatencyUnit,
};

use tracing::{error, info, instrument, warn, Span};

use crate::{cache::Cache, error, error::ApiError, Result};

const START_AMOUNT_PCT_PRECISION: u128 = 1_000;
const START_AMOUNT_PCT_DENOM: u128 = 100 * START_AMOUNT_PCT_PRECISION;

/// Calculate claimable amount for a user who hasn't called new_claim yet
/// This mirrors the logic from ClaimStatus::update_unlocked_amount_claimed
pub fn calculate_claimable_amount_for_new_user(
    unlocked_amount: u64,
    curr_ts: i64,
    start_ts: i64,
    end_ts: i64,
    start_amount_pct: u128,
) -> u64 {
    if curr_ts < start_ts {
        return 0;
    }

    if curr_ts >= end_ts {
        return unlocked_amount;
    }

    // Calculate vested amount following the same logic as update_unlocked_amount_claimed
    let time_into_unlock = (curr_ts - start_ts) as u128;
    let total_unlock_time = (end_ts - start_ts) as u128;

    if total_unlock_time == 0 {
        return unlocked_amount;
    }

    // Start amount is based on start_amount_pct (typically 50%)
    let start_amount_pct_num = start_amount_pct * START_AMOUNT_PCT_PRECISION;
    let start_amount =
        ((unlocked_amount as u128) * start_amount_pct_num / START_AMOUNT_PCT_DENOM) as u64;

    // Calculate bonus amount (linear vesting of the remaining portion)
    let bonus_amount =
        (time_into_unlock * (unlocked_amount - start_amount) as u128 / total_unlock_time) as u64;

    start_amount + bonus_amount
}

/// Calculate claimable amount for locked tokens (linear vesting, no start percentage)
/// This mirrors the logic from ClaimStatus::unlocked_amount
pub fn calculate_locked_amount_claimable(
    locked_amount: u64,
    curr_ts: i64,
    start_ts: i64,
    end_ts: i64,
) -> u64 {
    if curr_ts < start_ts {
        return 0;
    }

    if curr_ts >= end_ts {
        return locked_amount;
    }

    let time_into_unlock = (curr_ts - start_ts) as u128;
    let total_unlock_time = (end_ts - start_ts) as u128;

    if total_unlock_time == 0 {
        return locked_amount;
    }

    // Linear vesting of the full locked amount
    (time_into_unlock * locked_amount as u128 / total_unlock_time) as u64
}

pub struct RouterState {
    pub basic_auth_user: Option<String>,
    pub basic_auth_password: Option<String>,
    pub program_id: Pubkey,
    pub tree: HashMap<Pubkey, (Pubkey, TreeNode)>,
    pub rpc_client: RpcClient,
    pub cache: Cache,
    pub start_amount_pct: u128,
}

impl Debug for RouterState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouterState")
            .field("program_id", &self.program_id)
            .field("tree", &self.tree.len())
            .finish()
    }
}

impl RouterState {
    fn needs_auth(&self) -> bool {
        self.basic_auth_user.is_some() && self.basic_auth_password.is_some()
    }
}

#[instrument(level = "info")]
pub fn get_routes(state: Arc<RouterState>) -> Router {
    let middleware = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(error::handle_error))
        .layer(BufferLayer::new(10000))
        .layer(RateLimitLayer::new(10000, Duration::from_secs(1)))
        .layer(TimeoutLayer::new(Duration::from_secs(20)))
        .layer(LoadShedLayer::new())
        .layer(
            TraceLayer::new_for_http()
                .on_request(|request: &Request<Body>, _span: &Span| {
                    info!("started {} {}", request.method(), request.uri().path())
                })
                .on_response(
                    DefaultOnResponse::new()
                        .level(tracing_core::Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                ),
        );

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let mut router = Router::new().route("/", get(root));

    if state.needs_auth() {
        let auth_routes = Router::new()
            .route("/distributors", get(get_distributors))
            .route("/user/:user_pubkey", get(get_user_info))
            .route("/claim/:user_pubkey", get(get_claim_status))
            .route("/eligibility/:user_pubkey", get(get_eligibility))
            .route_layer(ValidateRequestHeaderLayer::basic(
                state.basic_auth_user.clone().unwrap().as_str(),
                state.basic_auth_password.clone().unwrap().as_str(),
            ));
        router = router.merge(auth_routes);
    } else {
        router = router
            .route("/distributors", get(get_distributors))
            .route("/user/:user_pubkey", get(get_user_info))
            .route("/claim/:user_pubkey", get(get_claim_status))
            .route("/eligibility/:user_pubkey", get(get_eligibility));
    }

    router.layer(middleware).layer(cors).with_state(state)
}

fn get_user_proof(
    merkle_tree: &HashMap<Pubkey, (Pubkey, TreeNode)>,
    pubkey: String,
) -> Result<UserProof> {
    let user_pubkey: Pubkey = Pubkey::from_str(pubkey.as_str())?;
    let node = merkle_tree
        .get(&user_pubkey)
        .ok_or(ApiError::UserNotFound(user_pubkey.to_string()))?;

    let proof = UserProof {
        merkle_tree: node.0.to_string(),
        amount: node.1.unlocked_amount(),
        locked_amount: node.1.locked_amount(),
        proof: node
            .1
            .proof
            .to_owned()
            .ok_or(ApiError::ProofNotFound(user_pubkey.to_string()))?,
    };
    Ok(proof)
}

/// Retrieve the proof for a given user
#[instrument(level = "error")]
async fn get_user_info(
    State(state): State<Arc<RouterState>>,
    Path(user_pubkey): Path<String>,
) -> Result<impl IntoResponse> {
    let merkle_tree = &state.tree;
    let proof = get_user_proof(merkle_tree, user_pubkey)?;
    Ok(Json(proof))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClaimStatusResp {
    /// Authority that claimed the tokens.
    pub claimant: Pubkey,
    /// Locked amount
    pub locked_amount: u64,
    /// Locked amount withdrawn
    pub locked_amount_withdrawn: u64,
    /// Unlocked amount
    pub unlocked_amount: u64,
    /// Unlocked amount claimed
    pub unlocked_amount_claimed: u64,
    /// indicate that whether admin can close this account, for testing purpose
    pub closable: bool,
    /// admin of merkle tree, store for for testing purpose
    pub distributor: Pubkey,
}

/// Retrieve the claim status for a user
#[instrument(level = "error")]
async fn get_claim_status(
    State(state): State<Arc<RouterState>>,
    Path(user_pubkey): Path<String>,
) -> Result<impl IntoResponse> {
    match state.cache.get_claim_status(&user_pubkey) {
        Some(data) => Ok(Json(ClaimStatusResp {
            claimant: data.data.claimant,
            locked_amount: data.data.locked_amount,
            locked_amount_withdrawn: data.data.locked_amount_withdrawn,
            unlocked_amount: data.data.unlocked_amount,
            unlocked_amount_claimed: data.data.unlocked_amount_claimed,
            closable: data.data.closable,
            distributor: data.data.distributor,
        })),
        None => Err(ApiError::UserNotFound(user_pubkey).into()),
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EligibilityResp {
    /// Authority to claim the token
    pub claimant: String,
    /// Address of the [MerkleDistributor] the claimant is in
    pub merkle_tree: String,
    /// Mint of token being distributed
    pub mint: String,
    /// Claim start time from [MerkleDistributor] (Unix Timestamp)
    pub start_ts: i64,
    /// Claim start time from [MerkleDistributor] (Unix Timestamp)
    pub end_ts: i64,
    /// Claimant's proof of inclusion in the Merkle Tree
    pub proof: Vec<[u8; 32]>,
    /// Amount user can claim at the beginning, start_amount = amount * START_PCT
    pub start_amount: u128,
    /// Amount user can claim at the end (max bonus)
    pub end_amount: u128,
    /// Amount excess of end_amount that is currently locked.
    pub unvested_amount: u128,
    /// Amount user has claimed, will be 0 if user has not claimed yet, this is the sum of unlocked_amount_claimed and locked_amount_withdrawn
    pub claimed_amount: u128,
    /// Amount user claimed out of their unlocked portion
    pub unlocked_amount_claimed: u128,
    /// Amount user claimed out of their locked portion
    pub locked_amount_withdrawn: u128,
    /// Amount user has locked
    pub locked_amount: u128,
    /// Amount user has unlocked so far
    pub claimable_amount: u128,
}

/// Retrieve the claim status for a user
#[instrument(level = "error")]
async fn get_eligibility(
    State(state): State<Arc<RouterState>>,
    Path(user_pubkey): Path<String>,
) -> Result<impl IntoResponse> {
    let merkle_tree = &state.tree;
    let proof = get_user_proof(merkle_tree, user_pubkey.clone())?;
    let distributor = state.cache.get_distributor(&proof.merkle_tree);
    let curr_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("epoch time")
        .as_secs() as i64;

    let (start_ts, end_ts, mint) = match distributor {
        Some(distributor) => (
            distributor.start_ts,
            distributor.end_ts,
            distributor.mint.to_string(),
        ),
        None => (
            state.cache.default_start_ts,
            state.cache.default_end_ts,
            state.cache.default_mint.clone(),
        ),
    };
    let (unlocked_amount_claimed, locked_amount_withdrawn, claimable_amount) = state
        .cache
        .get_claim_status(&user_pubkey)
        .map(|r| {
            (
                r.data.unlocked_amount_claimed,
                r.data.locked_amount_withdrawn,
                r.data
                    .amount_withdrawable(curr_ts, start_ts, end_ts)
                    .unwrap_or(0),
            )
        })
        .unwrap_or_else(|| {
            // No ClaimStatus exists - calculate what would be claimable
            // For unlocked amounts (proof.amount), use vesting with 50% start
            let unlocked_claimable = calculate_claimable_amount_for_new_user(
                proof.amount as u64,
                curr_ts,
                start_ts,
                end_ts,
                state.start_amount_pct,
            );

            // For locked amounts, calculate linear vesting (no 50% start)
            let locked_claimable = calculate_locked_amount_claimable(
                proof.locked_amount as u64,
                curr_ts,
                start_ts,
                end_ts,
            );

            (0, 0, unlocked_claimable + locked_claimable)
        });

    let start_amount_pct_num = state.start_amount_pct * START_AMOUNT_PCT_PRECISION;
    let start_amount = (proof.amount as u128)
        .checked_mul(start_amount_pct_num)
        .ok_or_else(|| {
            let err = ApiError::MathError();
            error!(
                "Math error occurred (1), amount: {}, START_AMOUNT_PCT_NUM: {}, START_AMOUNT_PCT_DENOM: {}",
                proof.amount, start_amount_pct_num, START_AMOUNT_PCT_DENOM
            );
            err
        })?
        .checked_div(START_AMOUNT_PCT_DENOM)
        .ok_or_else(|| {
            let err = ApiError::MathError();
            error!(
                "Math error occurred (2), amount: {}, START_AMOUNT_PCT_NUM: {}, START_AMOUNT_PCT_DENOM: {}",
                proof.amount, start_amount_pct_num, START_AMOUNT_PCT_DENOM
            );
            err
        })?;

    let unvested_amount = state.cache.get_unvested_amount(user_pubkey.clone());
    Ok(Json(EligibilityResp {
        claimant: user_pubkey.clone(),
        merkle_tree: proof.merkle_tree,
        start_ts,
        end_ts,
        mint,
        proof: proof.proof,
        start_amount,
        end_amount: proof.amount as u128,
        locked_amount: if proof.locked_amount as u128 > 0 {
            proof.locked_amount as u128
        } else {
            unvested_amount
        },
        claimable_amount: claimable_amount as u128,
        unvested_amount,
        claimed_amount: (unlocked_amount_claimed + locked_amount_withdrawn) as u128,
        unlocked_amount_claimed: unlocked_amount_claimed as u128,
        locked_amount_withdrawn: locked_amount_withdrawn as u128,
    }))
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SingleDistributor {
    pub distributor_pubkey: String,
    // pub merkle_root: [u8; 32],
    pub airdrop_version: u64,
    pub max_num_nodes: u64,
    pub max_total_claim: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DistributorAccount {
    /// Version of the airdrop
    pub version: u64,
    /// The 256-bit merkle root.
    pub root: [u8; 32],
    /// [Mint] of the token to be distributed.
    pub mint: Pubkey,
    /// Token Address of the vault
    pub token_vault: Pubkey,
    /// Maximum number of tokens that can ever be claimed from this [MerkleDistributor].
    pub max_total_claim: u64,
    /// Maximum number of nodes in [MerkleDistributor].
    pub max_num_nodes: u64,
    /// Total amount of tokens that have been claimed.
    pub total_amount_claimed: u64,
    /// Total amount of tokens that have been forgone.
    pub total_amount_forgone: u64,
    /// Number of nodes that have been claimed.
    pub num_nodes_claimed: u64,
    /// Lockup time start (Unix Timestamp)
    pub start_ts: i64,
    /// Lockup time end (Unix Timestamp)
    pub end_ts: i64,
    /// Clawback start (Unix Timestamp)
    pub clawback_start_ts: i64,
    /// Clawback receiver
    pub clawback_receiver: Pubkey,
    /// Admin wallet
    pub admin: Pubkey,
    /// Whether or not the distributor has been clawed back
    pub clawed_back: bool,
    /// this merkle tree is enable from this slot
    pub enable_slot: u64,
    /// indicate that whether admin can close this pool, for testing purpose
    pub closable: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MerkleDistributorResp {
    /// pubkey of the distributor
    pub pubkey: String,
    /// Version of the airdrop
    pub version: u64,
    // /// The 256-bit merkle root.
    // pub root: [u8; 32],
    /// [Mint] of the token to be distributed.
    pub mint: String,
    /// Token Address of the vault
    pub token_vault: String,
    /// Maximum number of tokens that can ever be claimed from this [MerkleDistributor].
    pub max_total_claim: u64,
    /// Maximum number of nodes in [MerkleDistributor].
    pub max_num_nodes: u64,
    /// Total amount of tokens that have been claimed.
    pub total_amount_claimed: u64,
    /// Total amount of tokens that have been forgone.
    pub total_amount_forgone: u64,
    /// Number of nodes that have been claimed.
    pub num_nodes_claimed: u64,
    /// Lockup time start (Unix Timestamp)
    pub start_ts: i64,
    /// Lockup time end (Unix Timestamp)
    pub end_ts: i64,
    /// Clawback start (Unix Timestamp)
    pub clawback_start_ts: i64,
    /// Clawback receiver
    pub clawback_receiver: String,
    /// Admin wallet
    pub admin: String,
    /// Whether or not the distributor has been clawed back
    pub clawed_back: bool,
    /// this merkle tree is enable from this slot
    pub enable_slot: u64,
    /// indicate that whether admin can close this pool, for testing purpose
    pub closable: bool,
}

impl MerkleDistributorResp {
    fn from(pubkey: String, distributor: MerkleDistributor) -> Self {
        MerkleDistributorResp {
            pubkey,
            version: distributor.version,
            // root: distributor.root,
            mint: distributor.mint.to_string(),
            token_vault: distributor.token_vault.to_string(),
            max_total_claim: distributor.max_total_claim,
            max_num_nodes: distributor.max_num_nodes,
            total_amount_claimed: distributor.total_amount_claimed,
            total_amount_forgone: distributor.total_amount_forgone,
            num_nodes_claimed: distributor.num_nodes_claimed,
            start_ts: distributor.start_ts,
            end_ts: distributor.end_ts,
            clawback_start_ts: distributor.clawback_start_ts,
            clawback_receiver: distributor.clawback_receiver.to_string(),
            admin: distributor.admin.to_string(),
            clawed_back: distributor.clawed_back,
            enable_slot: distributor.enable_slot,
            closable: distributor.closable,
        }
    }
}

async fn get_distributors(State(state): State<Arc<RouterState>>) -> Result<impl IntoResponse> {
    Ok(Json(
        state
            .cache
            .get_all_distributors()
            .into_iter()
            .map(|(k, v)| MerkleDistributorResp::from(k, v))
            .collect::<Vec<MerkleDistributorResp>>(),
    ))
}

async fn root() -> impl IntoResponse {
    "hey what u doing here"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_claimable_amount_before_start() {
        let amount = calculate_claimable_amount_for_new_user(1_000_000, 0, 10, 100, 50);
        assert_eq!(amount, 0);
    }

    #[test]
    fn test_calculate_claimable_amount_at_start() {
        // At start time, should get 50% (start_amount_pct = 50)
        let amount = calculate_claimable_amount_for_new_user(1_000_000, 10, 10, 100, 50);
        assert_eq!(amount, 500_000);
    }

    #[test]
    fn test_calculate_claimable_amount_halfway() {
        // Halfway through vesting period
        let amount = calculate_claimable_amount_for_new_user(1_000_000, 55, 10, 100, 50);
        // 50% start + 25% of remaining 50% = 75%
        assert_eq!(amount, 750_000);
    }

    #[test]
    fn test_calculate_claimable_amount_at_end() {
        // At end time, should get 100%
        let amount = calculate_claimable_amount_for_new_user(1_000_000, 100, 10, 100, 50);
        assert_eq!(amount, 1_000_000);
    }

    #[test]
    fn test_calculate_claimable_amount_after_end() {
        // After end time, should still get 100%
        let amount = calculate_claimable_amount_for_new_user(1_000_000, 200, 10, 100, 50);
        assert_eq!(amount, 1_000_000);
    }

    #[test]
    fn test_calculate_claimable_amount_zero_duration() {
        // Edge case: start_ts == end_ts
        let amount = calculate_claimable_amount_for_new_user(1_000_000, 10, 10, 10, 50);
        assert_eq!(amount, 1_000_000);
    }

    #[test]
    fn test_calculate_locked_amount_claimable() {
        // Test linear vesting for locked amounts
        let amount = calculate_locked_amount_claimable(1_000_000, 0, 10, 100);
        assert_eq!(amount, 0);

        let amount = calculate_locked_amount_claimable(1_000_000, 10, 10, 100);
        assert_eq!(amount, 0); // At start, 0% vested

        let amount = calculate_locked_amount_claimable(1_000_000, 55, 10, 100);
        assert_eq!(amount, 500_000); // Halfway through, 50% vested

        let amount = calculate_locked_amount_claimable(1_000_000, 100, 10, 100);
        assert_eq!(amount, 1_000_000); // At end, 100% vested
    }

    #[test]
    fn test_user_case_locked_amount() {
        // Test the specific case from the user
        // locked_amount: 750000000
        // start_ts: 1723809600
        // end_ts: 1747396800
        // current_ts: past end_ts

        let locked_amount = 750000000;
        let start_ts = 1723809600;
        let end_ts = 1747396800;
        let curr_ts = 1750149143; // Past end time

        let claimable = calculate_locked_amount_claimable(locked_amount, curr_ts, start_ts, end_ts);
        assert_eq!(claimable, 750000000); // Should be fully vested
    }
}
