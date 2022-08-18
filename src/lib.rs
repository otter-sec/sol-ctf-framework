// SPDX-License-Identifier: BSD-3-Clause
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::str::FromStr;
use solana_program_test::{ProgramTest, ProgramTestContext};

use solana_sdk::{
    signer::Signer,
    instruction::{AccountMeta, Instruction},
    signature::Keypair,
    pubkey::Pubkey,
};

use tempfile::NamedTempFile;

mod helpers {
    use solana_sdk::signature::Keypair;
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
    pub fn add_program(&mut self, path: &str, key: Option<Pubkey>) -> Pubkey {
        let program_so = std::fs::read(path).unwrap();
        let program_key = key.unwrap_or(helpers::keypair_from_data(&program_so).pubkey());

        self.builder.add_program(path, program_key, None);

        program_key
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

        let program_key = helpers::keypair_from_data(&input_so).pubkey();
        self.builder
            .add_program(&input_file.path().to_str().unwrap(), program_key, None);

        Ok(program_key)
    }
}

impl<R: BufRead, W: Write> Challenge<R, W> {
    pub fn builder(input: R, output: W) -> ChallengeBuilder<R, W> {
        let mut builder = ProgramTest::default();
        builder.prefer_bpf(true);

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
    pub fn read_instruction(
        &mut self,
        program_id: Pubkey,
    ) -> Result<Instruction, Box<dyn Error>> {
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

        let ix = Instruction::new_with_bytes(program_id, &ix_data, metas);

        Ok(ix)
    }
}

impl TryFrom<TcpStream> for ChallengeBuilder<BufReader<TcpStream>, TcpStream> {
    type Error = std::io::Error;

    fn try_from(socket: TcpStream) -> Result<Self, Self::Error> {
        let reader = BufReader::new(socket.try_clone()?);
        Ok(Challenge::builder(reader, socket))
    }
}
