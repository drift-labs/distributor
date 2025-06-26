use crate::*;

pub fn process_find_airdrop_version(args: &Args, new_distributor_args: &FindAirdropVersionArgs) {
    let program_client = args.get_program_client();
    let rpc_client = program_client.rpc();

    let start_version_check = new_distributor_args.start_airdrop_version.unwrap_or(0);
    let mut current_version = start_version_check;

    println!("Starting to find airdrop version for mint: {}, starting from: {}", args.mint, start_version_check);

    while current_version < 1000000000000000000 {
        let (distributor_pubkey, _bump) =
            get_merkle_distributor_pda(&args.program_id, &args.mint, current_version);
        match rpc_client.get_account_data(&distributor_pubkey) {
            Ok(_) => {
                println!("Airdrop version {} exists, account: {}", current_version, distributor_pubkey);
            }
            Err(e) => {
                if e.to_string().contains("AccountNotFound") {
                    println!("Airdrop version does not exist: {} <- next airdrop_version", current_version);
                    break;
                } else {
                    println!("Failed to get PDA, Error: {}", e.to_string());
                }
            }
        }
        current_version += 1;
    }
}