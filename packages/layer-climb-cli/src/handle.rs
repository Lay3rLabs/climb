use layer_climb::prelude::*;
use std::process::{Command, Stdio};

/// This is just a simple helper for running a Docker container with wasmd and cleaning up when done
/// useful for integration tests that need a chain running
///
/// More advanced use-cases with other chains or more control should use third-party tools
///
/// This instance represents a running Docker container. When dropped, it will attempt
/// to kill (and remove) the container automatically.
pub struct CosmosInstance {
    pub chain_config: ChainConfig,
    pub genesis_addresses: Vec<Address>,
    // the name for docker container and volume names, default is "climb-test-{chain_id}"
    pub name: String,
    // StdioKind::Null by default, can be set to StdioKind::Inherit to see logs
    pub stdout: StdioKind,
    // StdioKind::Null by default, can be set to StdioKind::Inherit to see logs
    pub stderr: StdioKind,
    // the block time to use in the chain, default is "200ms"
    pub block_time: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdioKind {
    Null,
    Inherit,
    Piped,
}

impl From<StdioKind> for Stdio {
    fn from(kind: StdioKind) -> Stdio {
        match kind {
            StdioKind::Null => Stdio::null(),
            StdioKind::Inherit => Stdio::inherit(),
            StdioKind::Piped => Stdio::piped(),
        }
    }
}

impl CosmosInstance {
    pub fn new(chain_config: ChainConfig, genesis_addresses: Vec<Address>) -> Self {
        Self {
            name: format!("climb-test-{}", chain_config.chain_id),
            chain_config,
            genesis_addresses,
            stdout: StdioKind::Null,
            stderr: StdioKind::Null,
            block_time: "200ms".to_string(),
        }
    }

    // simple all-in-one command
    // will return the block height that the chain is at when it is ready
    pub async fn start(&self) -> anyhow::Result<u64> {
        self.setup()?;
        self.run()?;
        self.wait_for_block().await
    }

    pub fn setup(&self) -> std::io::Result<()> {
        // first clean up any old instances
        self.clean();

        let mut args: Vec<String> = [
            "run",
            "--rm",
            "--name",
            &self.name,
            "--mount",
            &format!("type=volume,source={}_data,target=/root", self.name),
            "--env",
            &format!("CHAIN_ID={}", self.chain_config.chain_id),
            "--env",
            &format!("FEE={}", self.chain_config.gas_denom),
            "cosmwasm/wasmd:latest",
            "/opt/setup_wasmd.sh",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

        for addr in self.genesis_addresses.iter() {
            args.push(addr.to_string());
        }

        let res = Command::new("docker")
            .args(args)
            .stdout(self.stdout)
            .stderr(self.stderr)
            .spawn()?
            .wait()?;

        if !res.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to setup chain",
            ));
        }

        let res = Command::new("docker")
            .args([
                "run",
                "--rm",
                "--name",
                &self.name,
                "--mount",
                &format!("type=volume,source={}_data,target=/root", self.name),
                "cosmwasm/wasmd:latest",
                "sed",
                "-E",
                "-i",
                &format!(
                    "/timeout_(propose|prevote|precommit|commit)/s/[0-9]+m?s/{}/",
                    self.block_time
                ),
                "/root/.wasmd/config/config.toml",
            ])
            .stdout(self.stdout)
            .stderr(self.stderr)
            .spawn()?
            .wait()?;

        if !res.success() {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to setup chain",
            ))
        } else {
            Ok(())
        }
    }

    pub fn run(&self) -> std::io::Result<()> {
        let mut ports = vec![("26656", "26656"), ("1317", "1317")];

        if let Some(rpc_endpoint) = &self.chain_config.rpc_endpoint {
            let rpc_port = rpc_endpoint
                .split(':')
                .last()
                .expect("could not get rpc port");
            ports.push((rpc_port, "26657"));
        }

        if let Some(grpc_endpoint) = &self.chain_config.grpc_endpoint {
            let grpc_port = grpc_endpoint
                .split(':')
                .last()
                .expect("could not get grpc port");
            ports.push((grpc_port, "9090"));
        }

        let mut args: Vec<String> = ["run", "-d", "--name", &self.name]
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        for (host_port, container_port) in ports {
            args.push("-p".to_string());
            args.push(format!("{}:{}", host_port, container_port));
        }

        args.extend_from_slice(
            [
                "--mount",
                &format!("type=volume,source={}_data,target=/root", &self.name),
                "cosmwasm/wasmd:latest",
                "/opt/run_wasmd.sh",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .as_slice(),
        );

        let res = Command::new("docker").args(args).spawn()?.wait()?;

        if !res.success() {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to setup chain",
            ))
        } else {
            Ok(())
        }
    }

    pub async fn wait_for_block(&self) -> anyhow::Result<u64> {
        let query_client = QueryClient::new(
            self.chain_config.clone(),
            Some(Connection {
                preferred_mode: Some(ConnectionMode::Rpc),
                ..Default::default()
            }),
        )
        .await?;

        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let block_height = query_client.block_height().await.unwrap_or_default();
                if block_height > 0 {
                    break block_height;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
        })
        .await
        .map_err(|_| anyhow::anyhow!("Timeout waiting for block"))
    }

    pub fn clean(&self) {
        if let Ok(mut child) = std::process::Command::new("docker")
            .args(["kill", &self.name])
            .stdout(self.stdout)
            .stderr(self.stderr)
            .spawn()
        {
            let _ = child.wait();
        }

        if let Ok(mut child) = Command::new("docker")
            .args(["rm", &self.name])
            .stdout(self.stdout)
            .stderr(self.stderr)
            .spawn()
        {
            let _ = child.wait();
        }

        if let Ok(mut child) = Command::new("docker")
            .args(["volume", "rm", "-f", &format!("{}_data", self.name)])
            .stdout(self.stdout)
            .stderr(self.stderr)
            .spawn()
        {
            let _ = child.wait();
        }
    }
}

impl Drop for CosmosInstance {
    fn drop(&mut self) {
        self.clean();
    }
}
