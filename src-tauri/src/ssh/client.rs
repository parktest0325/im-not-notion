use anyhow::{Context, Result};
use std::fs::File;
use std::io::prelude::*;
use std::{net::TcpStream, path::Path};

use ssh2::Session;

pub struct Client {
    host: String,
    port: String,
    username: String,
    password: String,
    key_path: String,
    session: Session,
}

impl Client {
    pub fn new(
        host: &str,
        port: &str,
        username: &str,
        password: &str,
        key_path: &str,
    ) -> Result<Self> {
        let session = Session::new()?;
        Ok(Self {
            host: host.to_string(),
            port: port.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            key_path: key_path.to_string(),
            session,
        })
    }

    pub fn connect(&mut self) -> Result<()> {
        let tcp = TcpStream::connect(format!("{}:{}", self.host, self.port))
            .context("Failed to connect to host")?;
        self.session.set_tcp_stream(tcp);
        self.session.handshake().context("SSH handshake failed")?;

        if !self.password.is_empty() {
            self.session
                .userauth_password(&self.username, &self.password)
                .context("SSH authentication failed by password")?;
        } else {
            self.session
                .userauth_pubkey_file(&self.username, None, Path::new(&self.key_path), None)
                .context("SSH authentication failed by sshkey")?;
        }
        Ok(())
    }

    pub fn execute(&self, command: &str) -> Result<String> {
        let mut channel = self
            .session
            .channel_session()
            .context("Failed to open a channel")?;
        channel.exec(command).context("Failed to execute command")?;
        let mut output = String::new();
        channel
            .read_to_string(&mut output)
            .context("Failed to read command output")?;
        println!("{}", output);
        channel.send_eof().context("Failed to send EOF")?;
        channel.wait_close().context("Failed to close channel")?;
        Ok(output)
    }

    pub fn upload_file(&self, local_path: &str, remote_path: &str) -> Result<()> {
        let mut file = File::open(local_path).context("Failed to open local file")?;
        let metadata = file.metadata().context("Failed to read file metadata")?;
        let file_size = metadata.len();

        let mut remote_file = self
            .session
            .scp_send(Path::new(remote_path), 0o644, file_size, None)
            .context("Failed to create remote file")?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .context("Failed to read local file")?;
        remote_file
            .write_all(&buffer)
            .context("Failed to write to remote file")?;

        // Close the channel and wait for the whole content to be transferred
        remote_file.send_eof().context("Failed to send EOF")?;
        remote_file.wait_eof().context("Failed to wait for EOF")?;
        remote_file.close().context("Failed to close remote file")?;
        remote_file
            .wait_close()
            .context("Failed to wait for close confirmation")?;

        Ok(())
    }
}
