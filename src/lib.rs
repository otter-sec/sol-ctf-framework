use std::error::Error;
use std::io::{BufRead, Write, BufReader};
use std::net::TcpStream;
use std::str::FromStr;

use poc_framework::solana_sdk::instruction::{AccountMeta, Instruction};
use poc_framework::solana_sdk::signature::Keypair;
use poc_framework::{Environment, LocalEnvironment, solana_sdk::pubkey::Pubkey, solana_transaction_status::EncodedConfirmedTransaction};
use tempfile::NamedTempFile;
pub struct Challenge<R: BufRead, W: Write> {
    input: R,
    output: W,
    pub env: LocalEnvironment
}

impl<R: BufRead, W: Write> Challenge<R, W> {
    pub fn new(input: R, output: W) -> Challenge<R, W> {
        let env = LocalEnvironment::builder().build();
        Challenge {
            input,
            output,
            env,
        }
    }

    /// Reads program from input and deploys on environment
    pub fn input_program(&mut self) -> Result<Pubkey, Box<dyn Error>> {
        let mut line = String::new();
        writeln!(self.output, "program len: ")?;
        self.input.read_line(&mut line)?;
        let len: usize = line.trim().parse()?;

        let mut input_so = vec![0; len];
        self.input.read_exact(&mut input_so)?;

        let mut input_file = NamedTempFile::new()?;
        input_file.write_all(&input_so)?;

        let program_address = self.env.deploy_program(input_file.path());
        Ok(program_address)
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

impl TryFrom<TcpStream> for Challenge<BufReader<TcpStream>, TcpStream> {
    type Error = std::io::Error;

    fn try_from(socket: TcpStream) -> Result<Self, Self::Error> {
        let reader = BufReader::new(socket.try_clone()?);
        Ok(Challenge::new(reader, socket))
    }
}