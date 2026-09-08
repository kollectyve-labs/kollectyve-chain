require("@nomicfoundation/hardhat-toolbox");
require("dotenv").config();

// Well-known development keys. They are funded in the chain's `dev` genesis preset
// (see runtime/src/genesis_config_presets.rs). NEVER use these outside a dev chain.
const ALITH_PK = "0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133";
const BALTATHAR_PK = "0x8075991ce870b93a8870eca0c0f91913d12f47948ca0fd25b49c6fa7cdbeee8b";

const accounts = process.env.PRIVATE_KEY
  ? [process.env.PRIVATE_KEY]
  : [ALITH_PK, BALTATHAR_PK];

/** @type import('hardhat/config').HardhatUserConfig */
module.exports = {
  solidity: {
    version: "0.8.28",
    settings: {
      // pallet-revive's REVM backend implements the Cancun instruction set
      // (TLOAD/TSTORE/MCOPY/BLOBHASH are all present). Do not raise this past
      // cancun without checking substrate/frame/revive/src/vm/evm/instructions.
      evmVersion: "cancun",
      optimizer: { enabled: true, runs: 200 },
    },
  },
  paths: {
    sources: "./src",
    tests: "./test",
    cache: "./cache",
    artifacts: "./artifacts",
  },
  networks: {
    // `../scripts/dev-chain.sh` serves this endpoint via the eth-rpc proxy.
    kollectyveLocal: {
      url: process.env.ETH_RPC_URL || "http://127.0.0.1:8545",
      chainId: 28000,
      accounts,
    },
  },
  mocha: {
    // Blocks are authored by Aura every 6s, so a deploy + a few calls can take a while.
    timeout: 120000,
  },
};
