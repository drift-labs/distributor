use std::collections::HashMap;

use crate::*;

pub fn process_create_merkle_tree(args: &Args, merkle_tree_args: &CreateMerkleTreeArgs) {
    // Find the next available airdrop version
    let mut start_airdrop_version = 0;
    if args.mint == Pubkey::default() {
        println!("Mint is not set, will start creating merkle trees from version 0");
    }
    if args.mint != Pubkey::default() && merkle_tree_args.start_airdrop_version.is_none() {
        println!("Finding next available airdrop version for mint: {}", args.mint);
        
        let program_client = args.get_program_client();
        let rpc_client = program_client.rpc();
        
        let mut current_version = 0;
        while current_version < 1000000000000000000 {
            let (distributor_pubkey, _bump) =
                get_merkle_distributor_pda(&args.program_id, &args.mint, current_version);
            match rpc_client.get_account_data(&distributor_pubkey) {
                Ok(_) => {
                    println!("Airdrop version {} exists", current_version);
                }
                Err(e) => {
                    if e.to_string().contains("AccountNotFound") {
                        println!("Found next available airdrop version: {}", current_version);
                        start_airdrop_version = current_version;
                        break;
                    } else {
                        println!("Failed to get PDA, Error: {}", e.to_string());
                    }
                }
            }
            current_version += 1;
        }
    } else if let Some(version) = merkle_tree_args.start_airdrop_version {
        start_airdrop_version = version;
        println!("Using provided start airdrop version: {}", start_airdrop_version);
    }
    let mut csv_entries = CsvEntry::new_from_file(&merkle_tree_args.csv_path).unwrap();

    // exclude test address if have

    if merkle_tree_args.should_include_test_list {
        let test_list = get_test_list()
            .into_iter()
            .map(|x| (x, true))
            .collect::<HashMap<_, _>>();
        let mut new_entries = vec![];
        for entry in csv_entries.iter() {
            if test_list.contains_key(&entry.pubkey) {
                continue;
            }
            new_entries.push(entry.clone());
        }
        println!(
            "trim test wallets from {} to {}",
            csv_entries.len(),
            new_entries.len()
        );
        csv_entries = new_entries;
    }

    let max_nodes_per_tree = merkle_tree_args.max_nodes_per_tree as usize;

    let base_path = &merkle_tree_args.merkle_tree_path;
    let mut airdrop_version = start_airdrop_version;
    while csv_entries.len() > 0 {
        let last_index = max_nodes_per_tree.min(csv_entries.len());
        let sub_tree = csv_entries[0..last_index].to_vec();
        csv_entries = csv_entries[last_index..csv_entries.len()].to_vec();

        // use airdrop_version as version
        let merkle_tree =
            AirdropMerkleTree::new_from_entries(sub_tree, airdrop_version, merkle_tree_args.decimals)
                .unwrap();

        let base_path_clone = base_path.clone();
        let path = base_path_clone
            .as_path()
            .join(format!("tree_{}.json", airdrop_version));

        merkle_tree.write_to_file(&path);
        airdrop_version += 1;
    }

    if merkle_tree_args.should_include_test_list {
        println!("create merkle tree for test claming version {}", airdrop_version);
        let test_list = get_test_list()
            .into_iter()
            .map(|x| CsvEntry {
                pubkey: x,
                amount: merkle_tree_args.amount,
                locked_amount: Some(0),
            })
            .collect::<Vec<CsvEntry>>();

        let merkle_tree =
            AirdropMerkleTree::new_from_entries(test_list, airdrop_version, merkle_tree_args.decimals as u32)
                .unwrap();
        let base_path_clone = base_path.clone();
        let path = base_path_clone
            .as_path()
            .join(format!("tree_{}.json", airdrop_version));

        merkle_tree.write_to_file(&path);
        airdrop_version += 1;
    }
    
    // Output the last airdrop version used
    println!("Last airdrop version created: {}", airdrop_version - 1);
}
