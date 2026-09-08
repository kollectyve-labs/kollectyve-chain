const { ethers, network } = require("hardhat");

async function main() {
  const [deployer] = await ethers.getSigners();
  const balance = await ethers.provider.getBalance(deployer.address);

  console.log(`network:  ${network.name}`);
  console.log(`deployer: ${deployer.address}`);
  console.log(`balance:  ${ethers.formatEther(balance)}`);

  if (balance === 0n) {
    throw new Error(
      `${deployer.address} has no funds. Is it endowed in the dev genesis preset?`
    );
  }

  const Storage = await ethers.getContractFactory("Storage");
  const storage = await Storage.deploy(7);
  await storage.waitForDeployment();

  const address = await storage.getAddress();
  console.log(`Storage deployed at: ${address}`);
  console.log(`retrieve() => ${await storage.retrieve()}`);

  const tx = await storage.store(42);
  const receipt = await tx.wait();
  console.log(`store(42) in block ${receipt.blockNumber}, gasUsed ${receipt.gasUsed}`);
  console.log(`retrieve() => ${await storage.retrieve()}`);
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
