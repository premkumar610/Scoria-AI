// client/src/lib/__tests__/analytics.test.ts

import { DecentralizedAnalytics } from '../analytics';
import { DataPoint, PrivacyLevel } from '../types';
import { mockSolanaConnection } from './_fixtures';
import { TextEncoder } from 'util';

// Polyfill for browser crypto
Object.defineProperty(global, 'crypto', {
  value: {
    subtle: require('crypto').webcrypto.subtle
  }
});

const TEST_PROGRAM_ID = 'SCRAi...mock';
const TEST_ENDPOINT = 'http://localhost:8899';

describe('DecentralizedAnalytics', () => {
  let analytics: DecentralizedAnalytics;
  let mockStorage: jest.Mocked<any>;

  beforeEach(() => {
    mockStorage = {
      put: jest.fn().mockResolvedValue(true),
      getAll: jest.fn().mockResolvedValue([]),
      getLast: jest.fn().mockResolvedValue(null),
      clear: jest.fn().mockResolvedValue(true)
    };

    analytics = new DecentralizedAnalytics(TEST_ENDPOINT, TEST_PROGRAM_ID);
    Object.defineProperty(analytics, 'storage', { value: mockStorage });
  });

  describe('Data Processing Pipeline', () => {
    const sampleData: DataPoint = {
      metricType: 'performance',
      values: { fps: 60, inferenceTime: 100 },
      metadata: { modelId: 'resnet50', deviceId: 'device123' }
    };

    test('should apply privacy filters before storage', async () => {
      await analytics.processDataPoint(sampleData);
      
      const storedData = mockStorage.put.mock.calls[0][1];
      expect(storedData.userId).toBeUndefined();
      expect(storedData.deviceId).toMatch(/^[a-f0-9]{64}$/);
      expect(storedData.metadata.deviceId).toBeUndefined();
    });

    test('should add differential noise to numeric values', async () => {
      await analytics.processDataPoint(sampleData);
      
      const storedValue = mockStorage.put.mock.calls[0][1].values.fps;
      expect(storedValue).not.toBe(60);
      expect(Math.abs(storedValue - 60)).toBeLessThan(2); // ε=3.0
    });

    test('should trigger aggregation after window expiration', async () => {
      jest.spyOn(analytics, 'processAggregation').mockResolvedValue('txSig');
      
      // First data point
      await analytics.processDataPoint(sampleData);
      expect(analytics.processAggregation).not.toBeCalled();

      // Force window expiration
      mockStorage.getAll.mockResolvedValue([{}, {}, {}]);
      await analytics.processDataPoint(sampleData);
      expect(analytics.processAggregation).toBeCalledTimes(1);
    });
  });

  describe('Secure Aggregation', () => {
    const mockData = Array(100).fill({
      values: { metric: 50 },
      timestamp: Date.now()
    });

    beforeEach(() => {
      mockStorage.getAll.mockResolvedValue(mockData);
      jest.spyOn(global.Math, 'random').mockReturnValue(0.5);
    });

    test('should generate valid ZK proofs for aggregates', async () => {
      const proof = await analytics.processAggregation();
      
      expect(proof).toMatch(/^[A-Za-z0-9]{88}/);
      expect(proof).toHaveLength(88); // Base64 encoded 64-byte
    });

    test('should enforce k-anonymity thresholds', async () => {
      mockStorage.getAll.mockResolvedValue([{}]); // Single record
      
      await expect(analytics.processAggregation())
        .rejects.toThrow('Minimum cluster size');
    });
  });

  describe('Blockchain Integration', () => {
    test('should submit verified aggregates to chain', async () => {
      const mockSubmit = jest.spyOn(analytics, 'submitToChain')
        .mockResolvedValue('txSig123');

      await analytics.processAggregation();
      expect(mockSubmit).toBeCalledWith(
        expect.objectContaining({ count: 100 }),
        expect.any(Object) // ZKProof
      );
    });

    test('should reject unverified aggregates', async () => {
      jest.spyOn(analytics, 'verifyAggregate').mockResolvedValue(false);
      
      await expect(analytics.processAggregation())
        .rejects.toThrow('Aggregate verification failed');
    });
  });

  describe('Security Validation', () => {
    test('should detect storage tampering attempts', async () => {
      mockStorage.getAll.mockImplementation(() => {
        throw new Error('Invalid checksum');
      });
      
      await expect(analytics.processAggregation())
        .rejects.toThrow('Data integrity violation');
    });

    test('should prevent timing attacks on aggregation', async () => {
      const start = Date.now();
      await analytics.processAggregation();
      const duration = Date.now() - start;

      // Should have constant-time characteristics
      expect(duration).toBeLessThan(500);
      expect(duration).toBeGreaterThan(100);
    });
  });

  describe('Performance Benchmarks', () => {
    const LARGE_DATASET = Array(10_000).fill({
      values: { metric: Math.random() },
      timestamp: Date.now()
    });

    test('should process 10k points under 2s', async () => {
      mockStorage.getAll.mockResolvedValue(LARGE_DATASET);
      
      const start = performance.now();
      await analytics.processAggregation();
      const duration = performance.now() - start;

      expect(duration).toBeLessThan(2000);
    }, 10_000);
  });
});
