// SPDX-License-Identifier: BSD-3-Clause
use solana_program_test::{read_file, BanksClientError, ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::Account,
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::{keypair::Keypair, Signer},
    system_program,
    sysvar::rent::Rent,
    transaction::Transaction,
};
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use tempfile::NamedTempFile;

mod helpers {
    use crate::Keypair;
    use rand::{prelude::StdRng, SeedableRng};
    use sha2::{Digest, Sha256};
    pub fn keypair_from_data(data: &[u8]) -> Keypair {
        let mut hash = Sha256::default();
        hash.update(&data);

        // panic here is probably fine since this should always be 32 bytes, regardless of user input
        let mut rng = StdRng::from_seed(hash.finalize()[..].try_into().unwrap());
        Keypair::generate(&mut rng)
    }
}

pub struct Challenge<R: BufRead, W: Write> {
    input: R,
    output: W,
    pub env: ProgramTestContext,
}

pub struct ChallengeBuilder<R: BufRead, W: Write> {
    input: R,
    output: W,
    pub builder: ProgramTest,
}

impl<R: BufRead, W: Write> ChallengeBuilder<R, W> {
    /// New Challenge Environment
    pub fn new(input: R, output: W) -> ChallengeBuilder<R, W> {
        ChallengeBuilder {
            input,
            output,
            builder: ProgramTest::default(),
        }
    }

    /// Build challenge environment
    pub async fn build(self) -> Challenge<R, W> {
        Challenge {
            input: self.input,
            output: self.output,
            env: self.builder.start_with_context().await,
        }
    }

    /// Adds programs to challenge environment
    ///
    /// Returns vector of program pubkeys, with positions corresponding to input slice
    pub async fn add_chall_programs(&mut self, programs: &[&str]) -> Vec<Pubkey> {
        let mut keys = vec![];
        for &path in programs {
            let program_so = std::fs::read(path).unwrap();
            let program_keypair = helpers::keypair_from_data(&program_so);
            keys.push(program_keypair.pubkey());
            self.add_program(path, program_keypair.pubkey());
        }

        keys
    }

    /// Adds a program to the challenge environment
    pub fn add_program(&mut self, path: &str, key: Pubkey) -> Pubkey {
        self.prefer_bpf(true);
        let data = read_file(&path);
        self.add_account(
            key,
            Account {
                lamports: Rent::default().minimum_balance(data.len()).min(1),
                data,
                owner: solana_sdk::bpf_loader::id(),
                executable: true,
                rent_epoch: 0,
            },
        );
        return key;
    }

    pub fn prefer_bpf(&mut self, opt: bool) {
        self.builder.prefer_bpf(opt);
    }

    /// Reads program from input and adds it to environment
    pub async fn input_program(&mut self) -> Result<Pubkey, Box<dyn Error>> {
        let mut line = String::new();
        writeln!(self.output, "program len: ")?;
        self.input.read_line(&mut line)?;
        let mut len: usize = line.trim().parse()?;
        if len > 100000 {
            len = 100000
        }
        let mut input_so = vec![0; len];
        self.input.read_exact(&mut input_so)?;

        let mut input_file = NamedTempFile::new()?;
        input_file.write_all(&input_so)?;

        let program_keypair = helpers::keypair_from_data(&input_so);
        self.prefer_bpf(true);
        let _program = self.add_program(
            input_file.path().to_str().unwrap(),
            program_keypair.pubkey(),
        );

        Ok(program_keypair.pubkey())
    }

    /// Takes an account and pubkey from input and adds it to the environment
    pub fn add_account(&mut self, keypair: Pubkey, account: Account) -> Pubkey {
        self.builder.add_account(keypair, account);
        return keypair;
    }

    /// Takes an address pubkey, number of starting lamports, owner pubkey, and a filename from input, then adds an account with that data to the builder
    pub async fn add_account_with_file_data(
        &mut self,
        address: Pubkey,
        lamports: u64,
        owner: Pubkey,
        filename: &str,
    ) -> Result<(), ()> {
        self.builder
            .add_account_with_file_data(address, lamports, owner, filename);
        Ok(())
    }

    pub async fn add_account_with_base64_data(
        &mut self,
        address: Pubkey,
        lamports: u64,
        owner: Pubkey,
        b64: &str,
    ) -> Result<(), ()> {
        self.builder
            .add_account_with_base64_data(address, lamports, owner, b64);
        Ok(())
    }

    pub async fn input_instruction(
        &mut self,
        program_id: Pubkey,
        times: u64,
        lamports: u64,
    ) -> Result<Vec<Instruction>, Box<dyn Error>> {
        let mut ixs = Vec::new();
        for _ in 0..times {
            let mut line = String::new();
            writeln!(self.output, "num accounts: ")?;
            self.input.read_line(&mut line)?;
            if line == "" {
                break;
            }
            let num_accounts: usize = line.trim().parse().unwrap();
            let mut metas = vec![];
            for _ in 0..num_accounts {
                line.clear();
                writeln!(self.output, "Ix: ").unwrap();
                self.input.read_line(&mut line)?;

                let mut it = line.trim().split(' ');
                let meta = it.next().ok_or("bad meta");
                let mut pubkey = || it.next().ok_or("Bad Public Key");
                let pubkey = pubkey();
                if pubkey == Err("Bad Public Key") {
                    writeln!(self.output, "Bad Public Key!").unwrap();
                } else {
                    let pubkey = Pubkey::try_from(pubkey.unwrap()).unwrap();

                    let is_signer = if meta.unwrap().find("s") != None {
                        true
                    } else {
                        false
                    };
                    let is_writable = if meta.unwrap().find("w") != None {
                        true
                    } else {
                        false
                    };
                    let is_executeable = if meta.unwrap().find("e") != None {
                        true
                    } else {
                        false
                    };
                    if is_writable {
                        metas.push(AccountMeta::new(pubkey, is_signer));
                    } else {
                        metas.push(AccountMeta::new_readonly(pubkey, is_signer));
                    }
                    self.add_account(
                        pubkey,
                        Account {
                            lamports,
                            data: vec![],
                            owner: system_program::id(),
                            executable: is_executeable,
                            rent_epoch: 100000000,
                        },
                    );
                }
            }
            let mut line = String::new();
            writeln!(self.output, "ix len: ")?;
            self.input.read_line(&mut line)?;
            let ix_data_len: usize = line.trim().parse().unwrap();
            let mut ix_data = vec![0; ix_data_len];

            self.input.read_exact(&mut ix_data)?;

            ixs.push(Instruction::new_with_bytes(program_id, &ix_data, metas));
        }
        Ok(ixs)
    }
}

impl<R: BufRead, W: Write> Challenge<R, W> {
    /// Reads a transaction as input, and executes it
    pub async fn process_transaction(&mut self, tr: Transaction) -> Result<(), BanksClientError> {
        self.env
            .banks_client
            .process_transaction_with_preflight(tr)
            .await
    }

    /// Reads a vector of transactions as input and executes them
    pub async fn process_transactions(
        &mut self,
        trs: &[Transaction],
    ) -> Result<(), BanksClientError> {
        for tr in trs.to_vec() {
            let _res = self.process_transaction(tr).await;
        }
        Ok(())
    }

    /// Gets an account balance from Pubkey
    pub async fn get_balance(&mut self, key: Pubkey) -> Result<u64, BanksClientError> {
        self.env.banks_client.get_balance(key).await
    }

    pub async fn process_instructions(
        &mut self,
        instr: &[Instruction],
        payer: &Keypair,
    ) -> Result<(), BanksClientError> {
        let mut tr: Transaction = Transaction::new_with_payer(instr, Some(&payer.pubkey()));
        tr.sign(&[payer], self.get_latest_blockhash().await);
        self.process_transaction(tr).await
    }

    pub async fn process_instructions_signed(
        &mut self,
        instr: &[Instruction],
        payer: &Keypair,
        signers: &[&Keypair],
    ) -> Result<(), BanksClientError> {
        let mut tr: Transaction = Transaction::new_with_payer(instr, Some(&payer.pubkey()));
        for &signer in signers {
            let hash = self.get_latest_blockhash().await;
            tr.sign(&[signer], hash)
        }
        tr.sign(&[payer], self.get_latest_blockhash().await);
        dbg!(&tr);
        self.process_transaction(tr).await
    }

    pub async fn process_instruction(
        &mut self,
        instr: Instruction,
        payer: &Keypair,
    ) -> Result<(), BanksClientError> {
        self.process_instructions(&[instr], payer).await
    }

    pub async fn process_instruction_signed(
        &mut self,
        instr: Instruction,
        payer: &Keypair,
        signers: &[&Keypair],
    ) -> Result<(), BanksClientError> {
        self.process_instructions_signed(&[instr], payer, signers)
            .await
    }

    pub async fn get_latest_blockhash(&mut self) -> Hash {
        self.env.banks_client.get_latest_blockhash().await.unwrap()
    }
}

impl TryFrom<TcpStream> for ChallengeBuilder<BufReader<TcpStream>, TcpStream> {
    type Error = std::io::Error;

    fn try_from(socket: TcpStream) -> Result<Self, Self::Error> {
        let reader = BufReader::new(socket.try_clone()?);
        Ok(ChallengeBuilder::new(reader, socket))
    }
}
