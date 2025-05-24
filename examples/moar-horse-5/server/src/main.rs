use sol_ctf_framework::ChallengeBuilder;

use solana_sdk::{
    pubkey::Pubkey,
    account::Account,
    signature::{Keypair, Signer},
};

use solana_program::system_program;

use std::{
    fs,
    io::Write,
    error::Error,
    net::{
        TcpListener,
        TcpStream
    },
};

use moar_horse::{
    create, get_horse, get_wallet,
};

#[tokio::main]  
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("0.0.0.0:5001")?;
    loop {
        let (stream, _) = listener.accept()?;
        // move each socket to a Tokio task
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream).await {
                eprintln!("handler error: {e}");
            }
        });
    }
}

async fn handle_connection(mut socket: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut builder = ChallengeBuilder::try_from(socket.try_clone().unwrap()).unwrap();

    // load programs
    let solve_pubkey = match builder.input_program() {
        Ok(pubkey) => pubkey,
        Err(e) => {
            writeln!(socket, "Error: cannot add solve program → {e}")?;
            return Ok(());
        }
    };
    let program_key = Pubkey::new_unique();
    let program_pubkey = builder.add_program(&"../challenge/moar_horse.so", Some(program_key)).expect("Duplicate pubkey supplied");

    // make user
    let user = Keypair::new();

    writeln!(socket, "program: {}", program_pubkey)?;
    writeln!(socket, "user: {}", user.pubkey())?;


    // add accounts and lamports
    let (horse, _) = get_horse(program_pubkey);

    // const TARGET_AMT: u64 = 100_000;
    // const INIT_BAL: u64 = 1_447_680 + 5_000 + 890_880 + 5_000;
    // const VAULT_BAL: u64 = 1_000_000;
    const TARGET_AMT: u64 = 9_000_000_000;
    const INIT_BAL: u64 =  2_000_000_000;
    const VAULT_BAL: u64 = 10_000_000_000;

    
    builder
        .builder
        .add_account(user.pubkey(), Account::new(INIT_BAL, 0, &system_program::ID));
    builder
        .builder
        .add_account(horse, Account::new(VAULT_BAL, 0, &system_program::ID));

    let mut challenge = builder.build().await;

    // create a horsewallet
    challenge.run_ixs_full(
        &[create(program_pubkey, user.pubkey())],
        &[&user],
        &user.pubkey(),
    ).await?;

    // run solve
    let ixs = challenge.read_instruction(solve_pubkey).unwrap();
    challenge.run_ixs_full(
        &[ixs],
        &[&user],
        &user.pubkey(),
    ).await?;

    // check solve
    let balance = challenge.ctx.banks_client.get_account(user.pubkey()).await?.unwrap().lamports;
    writeln!(socket, "lamports: {:?}", balance)?;

    if balance > TARGET_AMT {
        let flag = fs::read_to_string("flag.txt").unwrap();
        writeln!(socket, "hhhhhhhoooooooooooorrrrrrrrrrrrrrrssssssssssssssssseeeeeeeeeeeeeeeeeee\nFlag: {}", flag)?;
    }

    Ok(())
}
