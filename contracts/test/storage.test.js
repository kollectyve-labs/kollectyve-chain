const { expect } = require("chai");
const { ethers, network } = require("hardhat");

// These tests run against a LIVE local chain (kollectyve-node + eth-rpc), not the
// in-process Hardhat network: `../scripts/dev-chain.sh` must be running.
describe("Storage on pallet-revive", function () {
  let storage;
  let deployer;

  before(async function () {
    if (network.name === "hardhat") {
      throw new Error(
        "Run against the local chain: npx hardhat test --network kollectyveLocal"
      );
    }
    [deployer] = await ethers.getSigners();

    const balance = await ethers.provider.getBalance(deployer.address);
    expect(balance, `${deployer.address} is not funded in genesis`).to.be.greaterThan(0n);

    const Storage = await ethers.getContractFactory("Storage");
    storage = await Storage.deploy(7);
    await storage.waitForDeployment();
  });

  it("reports the expected chain id", async function () {
    const net = await ethers.provider.getNetwork();
    expect(net.chainId).to.equal(28000n);
  });

  it("deploys with code at its address", async function () {
    const address = await storage.getAddress();
    expect(await ethers.provider.getCode(address)).to.not.equal("0x");
  });

  it("returns the constructor value", async function () {
    expect(await storage.retrieve()).to.equal(7n);
  });

  it("stores a new value and emits NumberChanged", async function () {
    await expect(storage.store(42))
      .to.emit(storage, "NumberChanged")
      .withArgs(deployer.address, 7n, 42n);

    expect(await storage.retrieve()).to.equal(42n);
  });

  it("increments and charges gas", async function () {
    const tx = await storage.increment();
    const receipt = await tx.wait();

    expect(receipt.status).to.equal(1);
    expect(receipt.gasUsed).to.be.greaterThan(0n);
    expect(await storage.retrieve()).to.equal(43n);
  });

  it("estimates gas for a state-changing call", async function () {
    const estimate = await storage.store.estimateGas(99);
    expect(estimate).to.be.greaterThan(0n);
  });
});
