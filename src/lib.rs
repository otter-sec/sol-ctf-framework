// SPDX-License-Identifier: BSD-3-Clause
use std::error::Error;
use std::io::{BufRead, Write, BufReader};
use std::net::TcpStream;
use std::str::FromStr;

use poc_framework_osec::solana_sdk::signer::Signer;

use poc_framework_osec::LocalEnvironmentBuilder;
use poc_framework_osec::solana_sdk::instruction::{AccountMeta, Instruction};
use poc_framework_osec::solana_sdk::signature::Keypair;
use poc_framework_osec::{Environment, LocalEnvironment, solana_sdk::pubkey::Pubkey, solana_transaction_status::EncodedConfirmedTransaction};
use tempfile::NamedTempFile;

mod helpers {
    use sha2::{Digest, Sha256};
    use rand::{prelude::StdRng, SeedableRng};
    use poc_framework_osec::solana_sdk::signature::Keypair;

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
    pub env: LocalEnvironment,
}

pub struct ChallengeBuilder<R: BufRead, W: Write> {
    input: R,
    output: W,
    pub builder: LocalEnvironmentBuilder,
}

impl<R: BufRead, W:Write> ChallengeBuilder<R, W> {
    /// Build challenge environment
    pub fn build(mut self) -> Challenge<R, W> {
        Challenge {
            input: self.input,
            output: self.output,
            env: self.builder.build(),
        }
    }

    /// Adds programs to challenge environment
    /// 
    /// Returns vector of program pubkeys, with positions corresponding to input slice
    pub fn chall_programs(&mut self, programs: &[&str]) -> Vec<Pubkey> {
        let mut keys = vec![];
        for &path in programs {
            let program_so = std::fs::read(path).unwrap();
            let program_keypair = helpers::keypair_from_data(&program_so);

            self.builder.add_program(program_keypair.pubkey(), path);
            keys.push(program_keypair.pubkey());
        }

        keys
    }

    /// Reads program from input and adds it to environment
    pub fn input_program(&mut self) -> Result<Pubkey, Box<dyn Error>> {
        let mut line = String::new();
        writeln!(self.output, "program len: ")?;
        self.input.read_line(&mut line)?;
        let len: usize = line.trim().parse()?;

        let mut input_so = vec![0; len];
        self.input.read_exact(&mut input_so)?;

        let mut input_file = NamedTempFile::new()?;
        input_file.write_all(&input_so)?;

        let program_keypair = helpers::keypair_from_data(&input_so);
        self.builder.add_program(program_keypair.pubkey(), input_file);

        Ok(program_keypair.pubkey())
    }
}

impl<R: BufRead, W: Write> Challenge<R, W> {
    pub fn builder(input: R, output: W) -> ChallengeBuilder<R, W> {
        let builder = LocalEnvironment::builder();
        ChallengeBuilder {
            input,
            output,
            builder,
        }
    }

    /// Reads instruction accounts/data from input and sends in transaction to specified program
    /// 
    /// # Account Format:
    /// `[meta] [pubkey]`
    /// 
    /// `[meta]` - contains "s" if account is a signer, "w" if it is writable
    /// `[pubkey]` - the address of the account
    pub fn input_instruction(&mut self, program_id: Pubkey, signers: &[&Keypair]) -> Result<EncodedConfirmedTransaction, Box<dyn Error>> {
        let mut line = String::new();
        writeln!(self.output, "num accounts: ")?;
        self.input.read_line(&mut line)?;
        let num_accounts: usize = line.trim().parse()?;

        let mut metas = vec![];
        for _ in 0..num_accounts {
            line.clear();
            self.input.read_line(&mut line)?;

            let mut it = line.trim().split(' ');
            let meta = it.next().ok_or("bad meta")?;
            let pubkey = it.next().ok_or("bad pubkey")?;
            let pubkey = Pubkey::from_str(pubkey)?;

            let is_signer = meta.contains('s');
            let is_writable = meta.contains('w');

            if is_writable {
                metas.push(AccountMeta::new(pubkey, is_signer));
            } else {
                metas.push(AccountMeta::new_readonly(pubkey, is_signer));
            }
        }

        line.clear();
        writeln!(self.output, "ix len: ")?;
        self.input.read_line(&mut line)?;
        let ix_data_len: usize = line.trim().parse()?;
        let mut ix_data = vec![0; ix_data_len];

        self.input.read_exact(&mut ix_data)?;

        let ix = Instruction::new_with_bytes(
            program_id,
            &ix_data,
            metas
        );

        Ok(self.env.execute_as_transaction(&[ix], signers))
    }
}

impl TryFrom<TcpStream> for ChallengeBuilder<BufReader<TcpStream>, TcpStream> {
    type Error = std::io::Error;

    fn try_from(socket: TcpStream) -> Result<Self, Self::Error> {
        let reader = BufReader::new(socket.try_clone()?);
        Ok(Challenge::builder(reader, socket))
    }
}
