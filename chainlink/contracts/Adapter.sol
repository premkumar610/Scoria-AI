// SPDX-License-Identifier: AGPL-3.0-only
pragma solidity 0.8.21;
pragma experimental ABIEncoderV2;

import "@openzeppelin/contracts/access/Ownable2Step.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";

/**
 * @title SCORIA AI Oracle Adapter
 * @dev Decentralized oracle network for verifying AI operations
 *      with ZKP validation and multi-chain compatibility
 */
contract AIOracleAdapter is Ownable2Step {
    using ECDSA for bytes32;
    
    // Struct Definitions
    struct AIRequest {
        bytes32 modelHash;
        bytes inputData;
        uint256 minConsensus;
        uint256 reward;
        address payable requester;
        mapping(address => bytes) nodeSignatures;
        address[] responders;
        bytes32 finalResult;
        bool isFulfilled;
    }

    struct NodeInfo {
        uint256 stakeAmount;
        uint256 reputation;
        bool isAuthorized;
        uint256 lastActivity;
    }

    // State Variables
    uint256 public constant MIN_STAKE = 1 ether;
    uint256 public constant RESPONSE_WINDOW = 30 minutes;
    
    mapping(bytes32 => AIRequest) private requests;
    mapping(address => NodeInfo) public nodes;
    mapping(bytes32 => uint256) public modelPrices;
    
    uint256 private requestNonce;
    address private zkVerifier;
    
    // Events
    event RequestCreated(
        bytes32 indexed requestId,
        bytes32 modelHash,
        address indexed requester,
        uint256 reward
    );
    
    event ResponseSubmitted(
        bytes32 indexed requestId,
        address indexed node,
        bytes32 resultHash,
        bytes zkProof
    );
    
    event RequestFulfilled(
        bytes32 indexed requestId,
        bytes32 finalResult,
        uint256 paymentDistributed
    );

    // Modifiers
    modifier onlyAuthorizedNode() {
        require(nodes[msg.sender].isAuthorized, "Unauthorized node");
        require(nodes[msg.sender].stakeAmount >= MIN_STAKE, "Insufficient stake");
        _;
    }

    constructor(address _zkVerifier) {
        zkVerifier = _zkVerifier;
    }

    /**
     * @dev Submit AI inference request to oracle network
     * @param modelHash - BLAKE3 hash of AI model architecture
     * @param inputData - Preprocessed input data for inference
     * @param minConsensus - Minimum node consensus required
     */
    function submitRequest(
        bytes32 modelHash,
        bytes memory inputData,
        uint256 minConsensus
    ) external payable {
        require(msg.value >= modelPrices[modelHash], "Insufficient payment");
        require(minConsensus >= 3, "Minimum 3 nodes required");
        
        bytes32 requestId = keccak256(abi.encodePacked(
            block.timestamp, msg.sender, requestNonce
        ));
        
        AIRequest storage newRequest = requests[requestId];
        newRequest.modelHash = modelHash;
        newRequest.inputData = inputData;
        newRequest.minConsensus = minConsensus;
        newRequest.reward = msg.value;
        newRequest.requester = payable(msg.sender);
        newRequest.isFulfilled = false;
        
        requestNonce++;
        
        emit RequestCreated(requestId, modelHash, msg.sender, msg.value);
    }

    /**
     * @dev Submit node response with cryptographic proof
     * @param requestId - Target request identifier
     * @param resultHash - Hash of inference output
     * @param zkProof - Zero-knowledge proof of correct execution
     * @param nodeSignature - Node's cryptographic signature
     */
    function submitResponse(
        bytes32 requestId,
        bytes32 resultHash,
        bytes memory zkProof,
        bytes memory nodeSignature
    ) external onlyAuthorizedNode {
        AIRequest storage request = requests[requestId];
        require(!request.isFulfilled, "Request already fulfilled");
        require(block.timestamp < request.lastActivity + RESPONSE_WINDOW, "Response window closed");
        
        // Verify ZKP proof
        require(verifyZKProof(request.modelHash, request.inputData, resultHash, zkProof), "Invalid ZK proof");
        
        // Verify node signature
        bytes32 messageHash = keccak256(abi.encodePacked(requestId, resultHash));
        require(messageHash.recover(nodeSignature) == msg.sender, "Invalid signature");
        
        request.nodeSignatures[msg.sender] = nodeSignature;
        request.responders.push(msg.sender);
        
        // Update node reputation
        nodes[msg.sender].reputation += 10;
        nodes[msg.sender].lastActivity = block.timestamp;
        
        emit ResponseSubmitted(requestId, msg.sender, resultHash, zkProof);
        
        // Check consensus threshold
        if(request.responders.length >= request.minConsensus) {
            finalizeRequest(requestId, resultHash);
        }
    }

    /**
     * @dev Internal function to finalize consensus and distribute payments
     */
    function finalizeRequest(bytes32 requestId, bytes32 resultHash) private {
        AIRequest storage request = requests[requestId];
        require(!request.isFulfilled, "Already fulfilled");
        
        request.finalResult = resultHash;
        request.isFulfilled = true;
        
        // Distribute rewards
        uint256 paymentPerNode = request.reward / request.responders.length;
        for(uint256 i = 0; i < request.responders.length; i++) {
            payable(request.responders[i]).transfer(paymentPerNode);
        }
        
        emit RequestFulfilled(requestId, resultHash, request.reward);
    }

    /**
     * @dev Verify ZK proof using dedicated verifier contract
     */
    function verifyZKProof(
        bytes32 modelHash,
        bytes memory inputData,
        bytes32 resultHash,
        bytes memory zkProof
    ) private view returns (bool) {
        // Implementation depends on ZK verifier architecture
        (bool success, bytes memory data) = zkVerifier.staticcall(abi.encodeWithSignature(
            "verifyProof(bytes32,bytes,bytes32,bytes)",
            modelHash,
            inputData,
            resultHash,
            zkProof
        ));
        
        return success && abi.decode(data, (bool));
    }

    // Administrative Functions
    function updateModelPrice(bytes32 modelHash, uint256 price) external onlyOwner {
        modelPrices[modelHash] = price;
    }
    
    function authorizeNode(address node, uint256 minStake) external onlyOwner {
        nodes[node].isAuthorized = true;
        nodes[node].stakeAmount = minStake;
    }
    
    function slashMaliciousNode(address node, uint256 penalty) external onlyOwner {
        require(nodes[node].reputation > 0, "No reputation to slash");
        nodes[node].reputation -= 20;
        nodes[node].stakeAmount -= penalty;
    }
    
    function withdrawStake(uint256 amount) external {
        require(nodes[msg.sender].stakeAmount >= amount, "Insufficient stake");
        nodes[msg.sender].stakeAmount -= amount;
        payable(msg.sender).transfer(amount);
    }
}
