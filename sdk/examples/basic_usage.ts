// client/examples/basic_usage.ts

import { ScoriaClient } from '../src/lib/client';
import { SolanaWalletAdapter } from '../src/lib/solana';
import { generateKeyPair, encryptWithPublicKey } from '../src/lib/crypto';
import { DecentralizedAnalytics } from '../src/lib/analytics';
import { ModelConfig, ZKProver } from '../src/lib/zk';
import type { KeyPair, PrivacyLevel } from '../src/lib/types';

// Phase 1: Client Initialization
const config = {
  apiEndpoint: 'https://api.scoria.ai/v2',
  network: 'mainnet-beta',
  apiKey: 'your_api_key_here',
  enableAnalytics: true,
  privacyLevel: 'strict' as PrivacyLevel
};

const client = new ScoriaClient(config);
const wallet = new SolanaWalletAdapter();
const analytics = new DecentralizedAnalytics(client);
const zkProver = new ZKProver();

// Phase 2: Core Workflow
async function executeSCORIAWorkflow() {
  try {
    // 2.1 Wallet Connection
    if (!await wallet.connect()) {
      throw new Error('Wallet connection failed');
    }
    console.log(`Connected to wallet: ${wallet.publicKey}`);

    // 2.2 Cryptographic Setup
    const userKeys: KeyPair = await generateKeyPair();
    console.log('Generated cryptographic keys');

    // 2.3 Data Encryption
    const sensitiveData = new TextEncoder().encode('Private AI input');
    const encryptedData = await encryptWithPublicKey(
      sensitiveData,
      userKeys.publicKey
    );
    console.log('Encrypted payload:', encryptedData.ciphertext);

    // 2.4 Analytics Initialization
    await analytics.initialize(wallet);
    analytics.track('session_start', {
      device_type: 'desktop',
      os: 'Linux'
    });

    // 2.5 AI Model Execution
    const modelConfig: ModelConfig = {
      id: 'gpt-4x-web3',
      quantization: 'FP16',
      privacyLevel: 'strict',
      zkCircuit: 'inference-verification'
    };

    const { results, proof } = await client.executeModel({
      model: modelConfig,
      inputs: encryptedData,
      prover: zkProver
    });
    console.log('Model outputs:', results);
    console.log('ZK Proof:', proof.signature);

    // 2.6 Blockchain Interaction
    const modelUploadTx = await client.uploadModel({
      wallet,
      modelBuffer: results,
      encryptionKey: userKeys.publicKey,
      storage: 'arweave'
    });
    console.log('Model stored at:', modelUploadTx.arweaveId);

    // 2.7 Privacy-preserving Swap
    const swapResult = await client.privacySwap({
      sender: wallet,
      receiver: 'SCORIA_VAULT_ADDRESS',
      data: encryptedData,
      proof,
      fee: 0.001 // SOL
    });
    console.log('Private transaction hash:', swapResult.txHash);

  } catch (error) {
    console.error('Workflow execution failed:');
    if (error instanceof Error) {
      console.error(`${error.name}: ${error.message}`);
      analytics.track('error', {
        type: error.name,
        message: error.message.slice(0, 100)
      });
    }
    process.exit(1);
  } finally {
    await wallet.disconnect();
    analytics.flush();
    console.log('Resources cleaned up');
  }
}

// Phase 3: Execution
(async () => {
  console.time('Full workflow');
  await executeSCORIAWorkflow();
  console.timeEnd('Full workflow');
})();
