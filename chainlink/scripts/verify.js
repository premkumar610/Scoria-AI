// chainlink/scripts/verify.js

const { ethers } = require("hardhat");
const { utils } = require("@chainlink/contracts");
const axios = require("axios");
const crypto = require("crypto");

// Environment Configuration
const NETWORK_CONFIG = {
  mainnet: {
    chainId: 1,
    explorerApi: "https://api.etherscan.io/api",
    linkToken: "0x514910771AF9Ca656af840dff83E8264EcF986CA",
    oracleTemplate: "0x012AFbC698250e1D18E7f0446B0dEc9fF65235C1"
  },
  kovan: {
    chainId: 42,
    explorerApi: "https://api-kovan.etherscan.io/api",
    linkToken: "0xa36085F69e2889c224210F603D836748e7dC0088",
    oracleTemplate: "0x56dd6586DB0D08c6Ce7B2f2805af30016e6578C4"
  },
  rinkeby: {
    chainId: 4,
    explorerApi: "https://api-rinkeby.etherscan.io/api",
    linkToken: "0x01BE23585060835E02B77ef475b0Cc51aA1e0709",
    oracleTemplate: "0x7AFe1118E77878C4BE9b48f872E5C3A6DBAFdA62"
  }
};

async function verifyContract(contractAddress, constructorArgs) {
  const network = await ethers.provider.getNetwork();
  const config = NETWORK_CONFIG[network.name] || NETWORK_CONFIG.kovan;
  
  if (!process.env.EXPLORER_API_KEY) {
    throw new Error("EXPLORER_API_KEY environment variable required");
  }

  const postData = {
    apikey: process.env.EXPLORER_API_KEY,
    module: "contract",
    action: "verifyproxycontract",
    address: contractAddress,
    codeformat: "solidity-single-file",
    contractname: "contracts/CustomOracle.sol:CustomOracle",
    compilerversion: "v0.8.7+commit.e28d00a7",
    optimizationUsed: 1,
    runs: 200,
    constructorArguements: constructorArgs
  };

  try {
    const response = await axios.post(config.explorerApi, postData, {
      headers: { "Content-Type": "application/x-www-form-urlencoded" }
    });
    
    if (response.data.status !== "1") {
      throw new Error(`Verification failed: ${response.data.result}`);
    }
    
    return response.data;
  } catch (error) {
    throw new Error(`Verification error: ${error.response?.data?.result || error.message}`);
  }
}

async function main() {
  // Initialize secure environment
  if (!process.env.DEPLOYER_PK && !process.env.HSM_MODULE) {
    throw new Error("Secure wallet configuration required (HSM_MODULE or DEPLOYER_PK)");
  }

  const [deployer] = process.env.HSM_MODULE 
    ? await ethers.getSigners(process.env.HSM_MODULE)
    : await ethers.getSigners();

  const network = await ethers.provider.getNetwork();
  const config = NETWORK_CONFIG[network.name] || NETWORK_CONFIG.kovan;

  // 1. Verify Oracle Contract
  const oracleAddress = process.env.ORACLE_ADDRESS;
  const oracleVerification = await verifyContract(
    oracleAddress,
    ethers.utils.defaultAbiCoder.encode(["address"], [config.linkToken])
  );
  console.log(`Oracle verification TX: ${oracleVerification.result}`);

  // 2. Verify Consumer Contract
  const consumerAddress = process.env.CONSUMER_ADDRESS;
  const consumerConstructorArgs = ethers.utils.defaultAbiCoder.encode(
    ["address", "address", "bytes32"],
    [
      oracleAddress,
      config.linkToken,
      process.env.JOB_SPEC_ID
    ]
  );
  const consumerVerification = await verifyContract(
    consumerAddress,
    consumerConstructorArgs
  );
  console.log(`Consumer verification TX: ${consumerVerification.result}`);

  // 3. Validate Chainlink Job Spec
  const oracle = await ethers.getContractAt("CustomOracle", oracleAddress);
  const jobHash = crypto.createHash("sha256").update(process.env.JOB_SPEC_ID).digest("hex");
  
  const jobSpecValidation = await oracle.lookupJob(utils.formatBytes32String(jobHash));
  if (jobSpecValidation.payment.toString() === "0") {
    throw new Error("Job specification not properly configured");
  }
  console.log("Job specification validated:", jobSpecValidation);

  // 4. Validate Ownership
  const oracleOwner = await oracle.owner();
  if (oracleOwner.toLowerCase() !== deployer.address.toLowerCase()) {
    throw new Error("Ownership verification failed");
  }
  console.log("Ownership verified for:", deployer.address);

  // 5. Cross-check bytecode
  const deployedBytecode = await ethers.provider.getCode(oracleAddress);
  const compiledBytecode = await ethers.getDeployedCode("CustomOracle");
  
  if (deployedBytecode !== compiledBytecode) {
    throw new Error("Bytecode mismatch detected");
  }
  console.log("Bytecode verification successful");

  // Output verification summary
  console.log(`
  ===========================
  Verification Complete
  ---------------------------
  Network:        ${network.name} (${config.chainId})
  Oracle:         ${oracleAddress}
  Consumer:       ${consumerAddress}
  Job Spec ID:    ${process.env.JOB_SPEC_ID}
  Owner:          ${deployer.address}
  ===========================
  `);
}

main()
  .then(() => process.exit(0))
  .catch(error => {
    console.error("❌ Verification failed:", error.message);
    process.exit(2);
  });
