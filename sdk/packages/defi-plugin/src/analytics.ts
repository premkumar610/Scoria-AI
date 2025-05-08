// client/src/lib/analytics.ts

import { Connection, PublicKey, Transaction } from '@solana/web3.js';
import { 
  secureAggregate,
  generateZKProof,
  verifyOnChain,
  type ZKProof,
  type SecureAggregateConfig
} from './crypto';
import { IndexedDBStore } from './storage';
import { AnalyticsResult, DataPoint, PrivacyLevel } from './types';

const DEFAULT_AGGREGATION_WINDOW = 3600; // 1 hour in seconds

export class DecentralizedAnalytics {
  private connection: Connection;
  private storage: IndexedDBStore;
  private readonly programId: PublicKey;

  constructor(
    endpoint: string,
    programId: string,
    storeName = 'scoria-analytics'
  ) {
    this.connection = new Connection(endpoint);
    this.programId = new PublicKey(programId);
    this.storage = new IndexedDBStore(storeName, 1, [
      { name: 'datapoints', keyPath: 'timestamp' },
      { name: 'proofs', keyPath: 'blockHash' }
    ]);
  }

  // Core Analytics Pipeline
  async processDataPoint(data: DataPoint): Promise<void> {
    try {
      // 1. Privacy-preserving preprocessing
      const sanitized = this.applyPrivacyFilters(data);
      
      // 2. Local differential privacy
      const noised = this.addDifferentialNoise(sanitized);
      
      // 3. Secure local storage
      await this.storage.put('datapoints', {
        ...noised,
        timestamp: Date.now(),
        deviceHash: await this.getDeviceFingerprint()
      });
      
      // 4. Periodic secure aggregation
      if (await this.needsAggregation()) {
        await this.processAggregation();
      }
    } catch (error) {
      console.error('Analytics processing error:', error);
      throw new AnalyticsError('Failed to process data point', error);
    }
  }

  // Privacy-preserving Aggregation
  private async processAggregation(): Promise<string> {
    const data = await this.storage.getAll('datapoints');
    const config: SecureAggregateConfig = {
      privacyLevel: PrivacyLevel.Strict,
      zkCircuit: 'analytics-agg-circuit',
      clusterRadius: 5
    };

    // 1. Secure Multi-Party Computation
    const aggregate = await secureAggregate(data, config);
    
    // 2. Zero-Knowledge Proof Generation
    const proof = await generateZKProof(aggregate, {
      circuit: 'analytics',
      private: true
    });

    // 3. On-Chain Validation
    const txSignature = await this.submitToChain(aggregate, proof);
    
    // 4. Local Data Rotation
    await this.storage.clear('datapoints');
    await this.storage.put('proofs', {
      blockHash: proof.blockHash,
      txSignature,
      timestamp: Date.now()
    });

    return txSignature;
  }

  // Blockchain Integration
  private async submitToChain(
    data: AnalyticsResult,
    proof: ZKProof
  ): Promise<string> {
    const tx = new Transaction().add(
      await this.buildAnalyticsInstruction(data, proof)
    );

    const { signature } = await this.connection.sendTransaction(
      tx,
      [], // No signers needed for program-based verification
      {
        skipPreflight: false,
        preflightCommitment: 'confirmed'
      }
    );

    await this.connection.confirmTransaction(signature, 'confirmed');
    return signature;
  }

  // Privacy Engineering
  private applyPrivacyFilters(data: DataPoint): DataPoint {
    return {
      ...data,
      // k-anonymity generalization
      userId: undefined,
      deviceId: this.hashIdentifier(data.deviceId),
      // Data minimization
      metadata: this.redactSensitiveFields(data.metadata)
    };
  }

  private addDifferentialNoise(data: DataPoint): DataPoint {
    const epsilon = 3.0; // ε-differential privacy budget
    const scale = 1.0 / epsilon;
    
    return {
      ...data,
      numericValues: data.numericValues.map(v => 
        v + this.laplaceNoise(scale)
      ),
      categoricalValues: this.randomizeCategories(data.categoricalValues)
    };
  }

  // Cryptographic Utilities
  private async getDeviceFingerprint(): Promise<string> {
    const encoder = new TextEncoder();
    const hardwareData = encoder.encode(
      JSON.stringify(await this.getHardwareProfile())
    );
    return crypto.subtle.digest('SHA-256', hardwareData)
      .then(hash => bufferToHex(hash));
  }

  // Hardware-Accelerated Operations
  private async getHardwareProfile(): Promise<HardwareProfile> {
    const perf = window.performance?.memory;
    return {
      concurrency: navigator.hardwareConcurrency,
      deviceMemory: navigator.deviceMemory,
      gpu: await this.detectGPUInfo(),
      memory: {
        jsHeapSizeLimit: perf?.jsHeapSizeLimit,
        totalJSHeapSize: perf?.totalJSHeapSize
      }
    };
  }

  // Utility Functions
  private async needsAggregation(): Promise<boolean> {
    const lastAgg = await this.storage.getLast('proofs');
    return !lastAgg || (Date.now() - lastAgg.timestamp) > 
      (DEFAULT_AGGREGATION_WINDOW * 1000);
  }

  private laplaceNoise(scale: number): number {
    const u = Math.random() - 0.5;
    return -scale * Math.sign(u) * Math.log(1 - 2 * Math.abs(u));
  }
}

// Supporting Types
interface HardwareProfile {
  concurrency: number;
  deviceMemory?: number;
  gpu: GPUExtension;
  memory: {
    jsHeapSizeLimit?: number;
    totalJSHeapSize?: number;
  };
}

interface GPUExtension {
  vendor: string;
  renderer: string;
}

class AnalyticsError extends Error {
  constructor(message: string, public readonly originalError?: unknown) {
    super(message);
    this.name = 'AnalyticsError';
  }
}

// Web Worker Operations
const analyticsWorker = new ComlinkWorker<typeof import('./analytics.worker')>(
  new URL('./analytics.worker', import.meta.url)
);

export async function backgroundAnalysis(data: DataPoint[]): Promise<void> {
  await analyticsWorker.processBatch(data);
}

// Debug Utilities
if (import.meta.env.DEV) {
  window.__SCORIA_ANALYTICS_DEBUG__ = {
    forceAggregation: () => analytics.processAggregation(),
    inspectStorage: () => analytics.storage.getAll('datapoints'),
    purgeData: () => analytics.storage.clearAll()
  };
}
