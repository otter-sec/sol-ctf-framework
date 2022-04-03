use std:: {
    net::{TcpListener, TcpStream},
    error::Error,
};
use threadpool::ThreadPool;
use std::io::Write;
use std::env;
use sol_ctf_framework::ChallengeBuilder;
use poc_framework::Environment;
use poc_framework::solana_sdk::signature::Keypair;
use poc_framework::solana_sdk::signature::Signer;
use anchor_client::solana_sdk::system_instruction::transfer;

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
    let mut builder = ChallengeBuilder::try_from(socket.try_clone().unwrap()).unwrap();

    let solve_pubkey = builder.input_program().unwrap();
    let program_pubkey = builder.chall_programs(&["./examples/solfire/solfire.so"])[0];

    let user = Keypair::new();

    writeln!(socket, "program pubkey: {}", program_pubkey)?;
    writeln!(socket, "solve pubkey: {}", solve_pubkey)?;
    writeln!(socket, "user pubkey: {}", user.pubkey())?;

    const TARGET_AMT: u64 = 50_000;
    const INIT_BAL: u64 = 10;
    const VAULT_BAL: u64 = 1_000_000;

    let mut challenge = builder.build();

    challenge.env.execute_as_transaction(
        &[transfer(
            &challenge.env.payer().pubkey(),
            &user.pubkey(),
            INIT_BAL,
            ),
            transfer(
                &challenge.env.payer().pubkey(),
                &program_pubkey,
                VAULT_BAL,
                )
        ],
        &[&challenge.env.payer()],
        );

    challenge.input_instruction(solve_pubkey, &[&user]).unwrap();

    let balance = challenge.env.get_account(user.pubkey()).unwrap().lamports;

    writeln!(socket, "user bal: {:?}", balance)?;
    writeln!(socket, "vault bal: {:?}", challenge.env.get_account(program_pubkey).unwrap().lamports)?;

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
