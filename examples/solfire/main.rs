use sol_ctf_framework::ChallengeBuilder;
use solana_program_test::tokio::runtime::Runtime;
use solana_sdk::{account::Account, pubkey::Pubkey, signature::{Keypair, Signer}};
use solana_system_interface::program as system_program;
use std::{
    env,
    error::Error,
    io::Write,
    net::{TcpListener, TcpStream},
};
use threadpool::ThreadPool;

fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("0.0.0.0:8080")?;
    let pool = ThreadPool::new(4);
    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.execute(|| {
            handle_connection(stream).unwrap();
        });
    }
    Ok(())
}

fn handle_connection(mut socket: TcpStream) -> Result<(), Box<dyn Error>> {
    let runtime = Runtime::new()?;
    let mut builder = ChallengeBuilder::try_from(socket.try_clone().unwrap()).unwrap();

    let solve_pubkey = match builder.input_program() {
        Ok(pubkey) => pubkey,
        Err(e) => {
            writeln!(socket, "Error: cannot add solve program → {e}")?;
            return Ok(());
        }
    };
    let program_pubkey = builder
        .add_program("./examples/solfire/solfire.so", None)
        .expect("Duplicate challenge program");

    let user = Keypair::new();

    writeln!(socket, "program pubkey: {}", program_pubkey)?;
    writeln!(socket, "solve pubkey: {}", solve_pubkey)?;
    writeln!(socket, "user pubkey: {}", user.pubkey())?;

    let (vault, _) = Pubkey::find_program_address(&["vault".as_ref()], &program_pubkey);

    const TARGET_AMT: u64 = 50_000;
    const INIT_BAL: u64 = 10;
    const VAULT_BAL: u64 = 1_000_000;

    builder
        .builder
        .add_account(user.pubkey(), Account::new(INIT_BAL, 0, &system_program::ID));
    builder
        .builder
        .add_account(vault, Account::new(VAULT_BAL, 0, &system_program::ID));

    let mut challenge = runtime.block_on(builder.build());

    let solve_ix = match challenge.read_instruction(solve_pubkey) {
        Ok(ix) => ix,
        Err(e) => {
            writeln!(socket, "Error: cannot read solve instruction → {e}")?;
            return Ok(());
        }
    };

    let payer = challenge.ctx.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let signers: [&Keypair; 2] = [&payer, &user];
    if let Err(e) = runtime.block_on(challenge.run_ixs_full(&[solve_ix], &signers, &payer_pubkey))
    {
        writeln!(socket, "Error: transaction failed → {e}")?;
        return Ok(());
    }

    let balance = runtime
        .block_on(challenge.ctx.banks_client.get_account(user.pubkey()))?
        .unwrap()
        .lamports;

    writeln!(socket, "user bal: {:?}", balance)?;
    writeln!(
        socket,
        "vault bal: {:?}",
        runtime
            .block_on(challenge.ctx.banks_client.get_account(vault))?
            .unwrap()
            .lamports
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
