const { ethers } = require("hardhat");

// Quick health check of the eth-rpc proxy: chain id, head, dev account balances.
const DEV_ACCOUNTS = {
  alith: "0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac",
  baltathar: "0x3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0",
  charleth: "0x798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc",
  dorothy: "0x773539d4Ac0e786233D90A233654ccEE26a613D9",
  ethan: "0xFf64d3F6efE2317EE2807d223a0Bdc4c0c49dfDB",
};

async function main() {
  const net = await ethers.provider.getNetwork();
  const head = await ethers.provider.getBlockNumber();
  const gasPrice = (await ethers.provider.getFeeData()).gasPrice;

  console.log(`chainId:  ${net.chainId}`);
  console.log(`head:     #${head}`);
  console.log(`gasPrice: ${gasPrice}`);
  console.log("balances:");
  for (const [name, address] of Object.entries(DEV_ACCOUNTS)) {
    const balance = await ethers.provider.getBalance(address);
    console.log(`  ${name.padEnd(10)} ${address}  ${ethers.formatEther(balance)}`);
  }
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
