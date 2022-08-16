use sol_ctf_framework::ChallengeBuilder;
use solana_sdk::account::Account;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signer;
use solana_sdk::system_program;
use std::env;
use std::io::Write;
use std::{
    error::Error,
    net::{TcpListener, TcpStream},
};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    println!("Listener Created!");
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let handle = || handle_connection(stream);
        if let Err(_err) = handle().await {
            println!("Error {:?}", _err);
        }
        //handle().await;
        println!("Connection Recieved and Handled!")
    }
    Ok(())
}

async fn handle_connection(mut socket: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut builder = ChallengeBuilder::try_from(socket.try_clone().unwrap()).unwrap();
    builder.prefer_bpf(true);
    let solve_pubkey = builder.input_program().await?;
    let program_pubkey = builder.add_chall_programs(&["solfire.so"]).await[0];

    let user = Keypair::new();
    let payer = Keypair::new();

    writeln!(socket, "program pubkey: {}", program_pubkey)?;
    writeln!(socket, "solve pubkey: {}", solve_pubkey)?;
    writeln!(socket, "user pubkey: {}", user.pubkey())?;

    let (vault, _) = Pubkey::find_program_address(&["vault".as_ref()], &program_pubkey);

    const TARGET_AMT: u64 = 50_000;
    const INIT_BAL: u64 = 5_0000 + 10;
    const VAULT_BAL: u64 = 1_000_000;

    builder.add_account(
        user.pubkey(),
        Account {
            lamports: INIT_BAL,
            data: vec![],
            owner: system_program::id(),
            executable: false,
            rent_epoch: 1860482537,
        },
    );

    builder.add_account(
        payer.pubkey(),
        Account {
            lamports: INIT_BAL,
            data: vec![],
            owner: system_program::id(),
            executable: false,
            rent_epoch: 1860482537,
        },
    );

    builder.add_account(
        vault,
        Account {
            lamports: VAULT_BAL,
            data: vec![],
            owner: system_program::id(),
            executable: false,
            rent_epoch: 1860482537,
        },
    );

    let instrs = builder
        .input_instruction(solve_pubkey)
        .await
        .unwrap();
    let mut challenge = builder.build().await;
    let balance = challenge.get_balance(payer.pubkey()).await.unwrap();
    println!("balace: {}", balance);

    challenge
        .process_instructions_signed(&instrs, &payer, &[&user])
        .await
        .unwrap();

    challenge.env.set_sysvar(&solana_sdk::sysvar::rent::Rent {
        lamports_per_byte_year: 0,
        exemption_threshold: 0.,
        burn_percent: 0,
    });
    dbg!(challenge.env.banks_client.get_rent().await).unwrap();
    let balance = challenge.get_balance(user.pubkey()).await.unwrap();

    writeln!(socket, "user bal: {:?}", balance)?;
    writeln!(
        socket,
        "vault bal: {:?}",
        challenge.get_balance(vault).await.unwrap()
    )?;

    if balance > TARGET_AMT {
        writeln!(socket, "congrats!")?;
        if let Ok(flag) = env::var("FLAG") {
            writeln!(socket, "flag: {:?}", flag)?;
        } else {
            writeln!(socket, "flag not found, please contact admin")?;
        }
    }

    Ok(())
}
