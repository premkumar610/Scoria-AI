// test/Adapter.test.js

const { expect } = require("chai");
const { ethers } = require("hardhat");
const { utils } = require("@chainlink/contracts");

describe("SCORIA AI Oracle Adapter", function() {
  let adapter;
  let linkToken;
  let owner;
  let validator;
  let unauthorized;

  before(async () => {
    [owner, validator, unauthorized] = await ethers.getSigners();
    
    // Deploy LINK Token
    const LinkToken = await ethers.getContractFactory("LinkToken");
    linkToken = await LinkToken.deploy();
    
    // Deploy Adapter
    const Adapter = await ethers.getContractFactory("Adapter");
    adapter = await Adapter.deploy(
      linkToken.address,
      owner.address,
      utils.toUtf8Bytes("ai.inference.requests")
    );
    
    // Fund adapter with LINK
    await linkToken.transfer(adapter.address, ethers.utils.parseUnits("1000", 18));
  });

  describe("Initialization", () => {
    it("Should set correct initial state", async () => {
      expect(await adapter.owner()).to.equal(owner.address);
      expect(await adapter.linkToken()).to.equal(linkToken.address);
      expect(await adapter.jobSpecId()).to.equal(
        utils.formatBytes32String("ai.inference.requests")
      );
    });
  });

  describe("Access Control", () => {
    it("Should reject unauthorized validator updates", async () => {
      await expect(
        adapter.connect(unauthorized).addValidator(validator.address)
      ).to.be.revertedWith("Caller is not the owner");
    });

    it("Should enforce quorum requirements", async () => {
      await adapter.addValidator(validator.address);
      await expect(
        adapter.connect(validator).submitResponse(1, "0x1234")
      ).to.be.revertedWith("Quorum not met");
    });
  });

  describe("Request Lifecycle", () => {
    let requestId;

    beforeEach(async () => {
      const tx = await adapter.connect(owner).createRequest(
        utils.formatBytes32String("model:v3.2.1"),
        "0x" // Encrypted input data
      );
      const receipt = await tx.wait();
      requestId = receipt.events[0].args.id;
    });

    it("Should process valid responses", async () => {
      // Add second validator
      const validator2 = (await ethers.getSigners())[3];
      await adapter.addValidator(validator2.address);

      // First validator response
      await adapter.connect(validator).submitResponse(requestId, "0xabcd");
      
      // Second validator response
      await adapter.connect(validator2).submitResponse(requestId, "0xabcd");

      // Check finalized result
      const result = await adapter.requests(requestId);
      expect(result.completed).to.be.true;
      expect(result.result).to.equal("0xabcd");
    });

    it("Should detect response mismatch", async () => {
      await adapter.addValidator(validator.address);
      await adapter.connect(validator).submitResponse(requestId, "0x1234");
      
      await expect(
        adapter.connect(validator).submitResponse(requestId, "0x5678")
      ).to.be.revertedWith("Response mismatch");
    });
  });

  describe("Security Validation", () => {
    it("Should prevent reentrancy attacks", async () => {
      const MaliciousValidator = await ethers.getContractFactory("MaliciousValidator");
      const attacker = await MaliciousValidator.deploy(adapter.address);
      
      await adapter.addValidator(attacker.address);
      await expect(
        attacker.triggerReentrancy()
      ).to.be.revertedWith("ReentrancyGuard: reentrant call");
    });

    it("Should validate cryptographic signatures", async () => {
      const fakeRequestId = 999;
      const fakeResult = "0xdeadbeef";
      const signature = await owner.signMessage(
        ethers.utils.arrayify(
          ethers.utils.solidityKeccak256(
            ["uint256", "bytes"], 
            [fakeRequestId, fakeResult]
          )
        )
      );

      await expect(
        adapter.finalizeRequest(fakeRequestId, fakeResult, signature)
      ).to.be.revertedWith("Invalid signature");
    });
  });

  describe("Edge Cases", () => {
    it("Should handle maximum input size", async () => {
      const maxSizeData = "0x" + "ff".repeat(4096); // 8KB
      await expect(
        adapter.createRequest(
          utils.formatBytes32String("model:edge-case"),
          maxSizeData
        )
      ).to.emit(adapter, "RequestCreated");
    });

    it("Should reject expired requests", async () => {
      // Set short timeout
      await adapter.setTimeout(60);
      
      const tx = await adapter.createRequest(
        utils.formatBytes32String("model:timeout-test"),
        "0x"
      );
      const receipt = await tx.wait();
      const requestId = receipt.events[0].args.id;

      // Fast-forward time
      await ethers.provider.send("evm_increaseTime", [61]);
      await ethers.provider.send("evm_mine");

      await expect(
        adapter.submitResponse(requestId, "0x1234")
      ).to.be.revertedWith("Request expired");
    });
  });
});
