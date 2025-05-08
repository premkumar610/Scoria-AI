// chainlink/scripts/deploy.js

const { ethers } = require("hardhat");
const { utils } = require("@chainlink/contracts");
const LINK_ABI = require("@chainlink/contracts/abi/v0.8/LinkToken.json");
const Oracle_ABI = require("../contracts/abi/CustomOracle.json");

// Environment Configuration
const NETWORK_CONFIG = {
  mainnet: {
    linkToken: "0x514910771AF9Ca656af840dff83E8264EcF986CA",
    nodeAddress: process.env.CHAINLINK_NODE_OPERATOR,
    fee: utils.toWei("0.25", "ether"),
    fundAmount: utils.toWei("100", "ether")
  },
  kovan: {
    linkToken: "0xa36085F69e2889c224210F603D836748e7dC0088",
    nodeAddress: "0x74EcC8Bdeb76F2A56765b7D5Dc97E5d40fA6C4E1",
    fee: utils.toWei("0.1", "ether"),
    fundAmount: utils.toWei("10", "ether")
  }
};

async function main() {
  // Load Deployment Network
  const network = await ethers.provider.getNetwork();
  const config = NETWORK_CONFIG[network.name] || NETWORK_CONFIG.kovan;
  
  // Initialize Secure Wallet
  const [deployer] = await ethers.getSigners();
  console.log(`Deploying contracts with account: ${deployer.address}`);

  // 1. Deploy LINK Token (Skip for MainNet)
  if(!config.linkToken) {
    const LinkToken = await ethers.getContractFactory("LinkToken");
    const linkToken = await LinkToken.deploy();
    await linkToken.deployed();
    console.log(`LINK Token deployed to: ${linkToken.address}`);
    config.linkToken = linkToken.address;
  }

  // 2. Deploy Custom Oracle
  const Oracle = await ethers.getContractFactory("CustomOracle");
  const oracle = await Oracle.deploy(config.linkToken);
  await oracle.deployed();
  console.log(`Oracle deployed to: ${oracle.address}`);

  // 3. Configure Oracle Parameters
  const tx1 = await oracle.setFulfillmentPermission(config.nodeAddress, true);
  await tx1.wait(2);
  const tx2 = await oracle.setAuthorization(config.nodeAddress, true);
  await tx2.wait(2);
  console.log("Oracle permissions configured");

  // 4. Create Job Specification
  const jobSpec = {
    id: utils.toHex("scoriaAIJob"),
    payment: config.fee,
    contractAddress: oracle.address,
    initiator: { type: "runlog" },
    tasks: [
      { type: "httpget", params: { get: "https://api.scoria.ai/data" }},
      { type: "jsonparse", params: { path: "$.results" }},
      { type: "multiply", params: { times: 100 }},
      { type: "ethuint256" },
      { type: "ethtx" }
    ]
  };
  const jobTx = await oracle.createJobSpecification(
    jobSpec.id,
    jobSpec.payment,
    jobSpec.contractAddress,
    JSON.stringify(jobSpec.tasks)
  );
  await jobTx.wait(2);
  console.log(`Job Spec ${jobSpec.id} created`);

  // 5. Deploy AI Consumer Contract
  const AIConsumer = await ethers.getContractFactory("AIConsumer");
  const aiConsumer = await AIConsumer.deploy(
    oracle.address,
    config.linkToken,
    jobSpec.id
  );
  await aiConsumer.deployed();
  console.log(`AI Consumer deployed to: ${aiConsumer.address}`);

  // 6. Fund Consumer Contract
  const linkToken = new ethers.Contract(config.linkToken, LINK_ABI, deployer);
  const fundTx = await linkToken.transfer(
    aiConsumer.address,
    config.fundAmount
  );
  await fundTx.wait(2);
  console.log(`${config.fundAmount} LINK transferred to consumer`);

  // 7. Verify Contracts
  if(network.config.verify) {
    await hre.run("verify:verify", {
      address: oracle.address,
      constructorArguments: [config.linkToken],
    });
    await hre.run("verify:verify", {
      address: aiConsumer.address,
      constructorArguments: [
        oracle.address,
        config.linkToken,
        jobSpec.id
      ],
    });
  }

  // Output Verification Info
  console.log("\nDeployment Verification:");
  console.log(`export SCORIA_ORACLE_ADDRESS=${oracle.address}`);
  console.log(`export SCORIA_CONSUMER_ADDRESS=${aiConsumer.address}`);
  console.log(`export CHAINLINK_JOB_ID=${jobSpec.id}`);
}

main()
  .then(() => process.exit(0))
  .catch(error => {
    console.error(error);
    process.exit(1);
  });
