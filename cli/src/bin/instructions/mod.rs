pub mod process_claim;
pub mod process_new_claim;
pub use process_claim::*;
pub use process_new_claim::*;
pub mod proccess_close_distribitor;
pub use proccess_close_distribitor::*;
pub mod process_new_distributor;
pub use process_new_distributor::*;
pub mod process_clawback;
pub use process_clawback::*;
pub mod process_create_merkle_tree;
pub use process_create_merkle_tree::*;
pub mod process_set_admin;
pub use process_set_admin::*;
pub mod process_set_enable_slot;
pub use process_set_enable_slot::*;
pub mod process_set_enable_slot_by_time;
pub use process_set_enable_slot_by_time::*;
pub mod process_create_dummy_csv;
pub use process_create_dummy_csv::*;
pub mod process_extend_list;
pub use process_extend_list::*;
pub mod process_create_test_list;
pub use process_create_test_list::*;
pub mod process_fund_all;
pub use process_fund_all::*;
pub mod process_verify;
pub use process_verify::*;
pub mod process_filter_list;
pub use process_filter_list::*;
pub mod process_get_slot;
pub use process_get_slot::*;
pub mod process_close_claim_status;
pub use process_close_claim_status::*;
pub mod process_filter_and_merge;
pub use process_filter_and_merge::*;
pub mod process_generate_kv_proof;
pub use process_generate_kv_proof::*;
pub mod process_send;
pub use process_send::*;
pub mod verify_kv_proof;
pub use verify_kv_proof::*;
pub mod process_set_clawback_receiver;
pub use process_set_clawback_receiver::*;
pub mod process_find_airdrop_version;
pub use process_find_airdrop_version::*;
