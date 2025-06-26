use crate::*;

pub fn process_create_test_list(args: &Args, create_test_list_args: &CreateTestListArgs) {
    let pre_list = get_pre_list();
    let mut wtr = Writer::from_path(&create_test_list_args.csv_path).unwrap();
    wtr.write_record(&["pubkey", "amount"]).unwrap();

    for addr in pre_list.iter() {
        wtr.write_record(&[addr, &format!("{}", create_test_list_args.amount)])
            .unwrap();
    }
    wtr.flush().unwrap();

    let merkle_tree_args = &CreateMerkleTreeArgs {
        csv_path: create_test_list_args.csv_path.clone(),
        merkle_tree_path: create_test_list_args.merkle_tree_path.clone(),
        max_nodes_per_tree: create_test_list_args.amount,
        should_include_test_list: false,
        amount: create_test_list_args.amount,
        decimals: create_test_list_args.decimals,
        start_airdrop_version: None,
    };
    process_create_merkle_tree(args, merkle_tree_args);
}
