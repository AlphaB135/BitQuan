BitQuan reference node (prototype)

Usage: bitquan-node <COMMAND>

Commands:
  run                Runs a placeholder node loop
  mine-genesis       Mine the genesis block for BitQuan blockchain
  check-block        Validates a block provided via an external source (placeholder)
  rng                Generates random bytes and derived streams using the BitQuan RNG
  mine-once          Mines a single block template by iterating nonces up to a limit (demo CPU miner)
  mine               Continuous mining mode with persistent storage
  wallet-gen         Generates a post-quantum keypair for wallet
  wallet-address     Import/show wallet address from keypair file
  address-to-script  Convert Bech32m address to script hex for mining
  wallet-sign        Sign a message with wallet keypair
  wallet-verify      Verify a signature
  wallet-send        Send transaction from wallet
  build-tx           Builds a simple unsigned transaction (1-in, 1-out) and prints JSON
  p2p-demo           Run a local P2P handshake demo (server+client) on a TCP address
  p2p-server         Start a P2P server that accepts peer connections
  p2p-connect        Connect to a peer as a client
  balance            Check balance for a given script/address
  help               Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
Continuous mining mode with persistent storage

Usage: bitquan-node mine [OPTIONS]

Options:
      --datadir <DATADIR>
          Data directory for blockchain storage [default: ./data/chainstate]
      --payout-script-hex <PAYOUT_SCRIPT_HEX>
          Hex-encoded script_pubkey for coinbase payout [default: 76a9140088ac]
      --bits <BITS>
          Compact bits target (0 = auto-adjust from chain) [default: 0]
      --max-nonce <MAX_NONCE>
          Maximum nonce per block attempt [default: 100000000]
      --threads <THREADS>
          Number of threads for mining (0 = CPU count) [default: 1]
  -h, --help
          Print help
  -V, --version
          Print version
Generates a post-quantum keypair for wallet

Usage: bitquan-node wallet-gen [OPTIONS]

Options:
      --algo <ALGO>          Algorithm (dilithium3, falcon512, sphincs) [default: dilithium3]
      --output <OUTPUT>      Output file for keypair (optional)
      --password <PASSWORD>  Password to encrypt the keystore (interactive prompt if not provided)
  -h, --help                 Print help
  -V, --version              Print version
