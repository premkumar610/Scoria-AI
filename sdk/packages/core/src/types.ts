// client/src/lib/types.ts

/**
 * Blockchain Core Types
 */
export interface SolanaTransaction {
  signature: string;
  slot: number;
  blockTime: number;
  meta: {
    fee: number;
    computeUnits: number;
    success: boolean;
    logMessages: string[];
  };
}

export type AccountData<T> = {
  pubkey: string;
  lamports: number;
  executable: boolean;
  data: T;
};

export enum ContractState {
  Active = "ACTIVE",
  Deprecated = "DEPRECATED",
  Frozen = "FROZEN"
}

/**
 * AI Model Management
 */
export interface ModelMetadata {
  modelHash: string;
  owner: string;
  createdAt: number;
  versions: ModelVersion[];
  dependencies: DependencyGraph;
  license: LicenseType;
  storage: StorageLocation;
  accessControl: AccessPolicy[];
}

export type ModelVersion = {
  semver: string;
  ipfsCID: string;
  zkCircuitHash: string;
  timestamp: number;
  contributors: Contributor[];
};

export type DependencyGraph = {
  nodes: DependencyNode[];
  edges: DependencyEdge[];
};

export type DependencyNode = {
  package: string;
  version: string;
  integrity: string;
};

export type DependencyEdge = {
  from: string;
  to: string;
  constraint: string;
};

/**
 * Zero-Knowledge Proof System
 */
export interface ZKProofPackage {
  arithmetization: 'PLONK' | 'Groth16';
  curve: 'BN254' | 'BLS12_381';
  publicInputs: string[];
  proof: string;
  verificationKey: string;
  circuitHash: string;
}

export type ProofConfig = {
  maxDegree: number;
  challengeBits: number;
  transcriptType: 'Keccak' | 'Poseidon';
  trustedSetupHash?: string;
};

/**
 * Network Configuration
 */
export type ClusterConfig = {
  name: 'mainnet' | 'devnet' | 'testnet';
  rpcEndpoint: string;
  wsEndpoint: string;
  explorer: string;
  programIDs: {
    modelRegistry: string;
    inferenceEngine: string;
    governance: string;
  };
};

/**
 * Cryptographic Primitives
 */
export type KeypairBundle = {
  publicKey: string;
  encryptedSecret: string;
  derivationPath?: string;
  algorithm: 'ed25519' | 'secp256k1';
};

export type SignedMessage<T> = {
  payload: T;
  signature: string;
  timestamp: number;
  publicKey: string;
};

/**
 * API Response Structures
 */
export type PaginatedResponse<T> = {
  data: T[];
  total: number;
  limit: number;
  offset: number;
  hasMore: boolean;
};

export type ErrorResponse = {
  code: number;
  message: string;
  context?: {
    param?: string;
    stack?: string;
    txSignature?: string;
  };
};

/**
 * Hardware Acceleration
 */
export type ComputeConfig = {
  deviceType: 'CPU' | 'GPU' | 'TPU';
  vendor: 'NVIDIA' | 'AMD' | 'Intel';
  memoryAllocation: number;
  precision: 'FP32' | 'FP16' | 'INT8';
};

/**
 * Governance Types
 */
export type GovernanceProposal = {
  id: string;
  proposer: string;
  description: string;
  votingOptions: VotingOption[];
  startSlot: number;
  endSlot: number;
  state: 'pending' | 'active' | 'passed' | 'rejected';
};

export type VotingOption = {
  id: number;
  label: string;
  weight: number;
};

/**
 * Event Streaming
 */
export type EventStreamConfig = {
  type: 'websocket' | 'server-sent-events';
  reconnectStrategy: 'exponential' | 'linear';
  topics: string[];
  auth: {
    mechanism: 'jwt' | 'signature';
    refreshInterval: number;
  };
};

/**
 * Storage Types
 */
export type StorageLocation = {
  protocol: 'ipfs' | 'arweave' | 'aws-s3';
  uri: string;
  encryption: 'AES-GCM' | 'ECIES';
  integrityHash: string;
};

/**
 * Runtime Configuration
 */
export type RuntimeConstraints = {
  maxMemoryMB: number;
  timeoutSeconds: number;
  maxComputeUnits: number;
  allowNetworkAccess: boolean;
  allowPersistentStorage: boolean;
};

// Utility types
export type Nullable<T> = T | null;
export type DeepPartial<T> = {
  [P in keyof T]?: DeepPartial<T[P]>;
};
export type RequireAtLeastOne<T> = {
  [K in keyof T]-?: Required<Pick<T, K>> & Partial<Pick<T, Exclude<keyof T, K>>>;
}[keyof T];
