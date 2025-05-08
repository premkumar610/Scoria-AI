// client/src/lib/solana.ts

import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
  VersionedTransaction,
  Signer,
  SendOptions,
  ConfirmOptions,
  Commitment,
} from '@solana/web3.js';
import { WalletError, WalletNotConnectedError } from '@solana/wallet-adapter-base';
import {
  WalletAdapter,
  WalletAdapterEvents,
  WalletReadyState,
} from '@solana/wallet-adapter-base';
import { Wallet } from '@solana/wallet-standard';

export interface SolanaWalletConfig {
  endpoint: string;
  commitment?: Commitment;
  autoConnect?: boolean;
  eventHandlers?: WalletEventHandlers;
}

type WalletEventHandlers = {
  onConnect?: (publicKey: PublicKey) => void;
  onDisconnect?: () => void;
  onError?: (error: WalletError) => void;
  onTransaction?: (signature: string, result: any) => void;
};

export class SolanaWalletAdapter implements WalletAdapter, WalletAdapterEvents {
  private _connection: Connection;
  private _wallet: Wallet | null = null;
  private _publicKey: PublicKey | null = null;
  private _readyState: WalletReadyState = WalletReadyState.NotDetected;
  private _eventHandlers: WalletEventHandlers = {};
  private _sessionTimeout: NodeJS.Timeout | null = null;

  constructor(config: SolanaWalletConfig) {
    this._connection = new Connection(config.endpoint, {
      commitment: config.commitment || 'confirmed',
    });
    if (config.autoConnect) this.autoConnect();
    this._eventHandlers = config.eventHandlers || {};
  }

  // Core Wallet Adapter Implementation
  get publicKey(): PublicKey | null {
    return this._publicKey;
  }

  get connected(): boolean {
    return !!this._publicKey;
  }

  get readyState(): WalletReadyState {
    return this._readyState;
  }

  async connect(): Promise<void> {
    try {
      if (this.connected) return;

      const wallet = await this.detectWallet();
      if (!wallet) throw new WalletNotConnectedError();

      const { publicKey } = await wallet.connect();
      this._wallet = wallet;
      this._publicKey = new PublicKey(publicKey);
      this._readyState = WalletReadyState.Loadable;

      this.setupSessionTimer();
      this._eventHandlers.onConnect?.(this._publicKey);
    } catch (error) {
      this._eventHandlers.onError?.(error as WalletError);
      throw error;
    }
  }

  async disconnect(): Promise<void> {
    if (this._wallet) {
      await this._wallet.disconnect();
      this._wallet = null;
    }
    this._publicKey = null;
    this._readyState = WalletReadyState.NotDetected;
    this.clearSessionTimer();
    this._eventHandlers.onDisconnect?.();
  }

  async sendTransaction(
    transaction: Transaction | VersionedTransaction,
    options?: SendOptions
  ): Promise<string> {
    if (!this.connected || !this._wallet) throw new WalletNotConnectedError();

    try {
      const { signature } = await this._wallet.sendTransaction(
        transaction,
        this._connection,
        options
      );
      
      const confirmation = await this.confirmTransaction(
        signature,
        options?.commitment
      );
      
      this._eventHandlers.onTransaction?.(signature, confirmation);
      return signature;
    } catch (error) {
      this._eventHandlers.onError?.(error as WalletError);
      throw error;
    }
  }

  async signTransaction<T extends Transaction | VersionedTransaction>(
    transaction: T
  ): Promise<T> {
    if (!this.connected || !this._wallet) throw new WalletNotConnectedError();
    return this._wallet.signTransaction(transaction);
  }

  async signAllTransactions<T extends Transaction | VersionedTransaction>(
    transactions: T[]
  ): Promise<T[]> {
    if (!this.connected || !this._wallet) throw new WalletNotConnectedError();
    return this._wallet.signAllTransactions(transactions);
  }

  async signMessage(message: Uint8Array): Promise<Uint8Array> {
    if (!this.connected || !this._wallet) throw new WalletNotConnectedError();
    return this._wallet.signMessage(message);
  }

  // Advanced Features
  private async detectWallet(): Promise<Wallet | null> {
    if (typeof window === 'undefined') return null;

    const standardWallet = window.navigator.wallets?.find(
      (wallet) => wallet.name === 'Phantom' || wallet.name === 'Solflare'
    );

    return standardWallet || null;
  }

  private async confirmTransaction(
    signature: string,
    commitment?: Commitment
  ): Promise<any> {
    return this._connection.confirmTransaction(
      signature,
      commitment || this._connection.commitment
    );
  }

  private autoConnect(): void {
    if (typeof window !== 'undefined' && window.localStorage) {
      const cachedPublicKey = localStorage.getItem('scoria:wallet-pk');
      if (cachedPublicKey) {
        this._publicKey = new PublicKey(cachedPublicKey);
        this._readyState = WalletReadyState.Loadable;
      }
    }
  }

  private setupSessionTimer(): void {
    this.clearSessionTimer();
    this._sessionTimeout = setTimeout(
      () => this.disconnect(),
      30 * 60 * 1000 // 30-minute session
    );
  }

  private clearSessionTimer(): void {
    if (this._sessionTimeout) {
      clearTimeout(this._sessionTimeout);
      this._sessionTimeout = null;
    }
  }

  // Event Handling
  on<E extends keyof WalletAdapterEvents>(
    event: E,
    listener: WalletAdapterEvents[E]
  ): () => void {
    // Implementation for event emitter pattern
    return () => {};
  }
}

// Utility Functions
export async function createTransferInstruction(
  fromPubkey: PublicKey,
  toPubkey: PublicKey,
  amount: number,
  decimals: number = 9
): Promise<TransactionInstruction> {
  // Implementation for token transfer instruction
}

export function validateAddress(address: string): boolean {
  try {
    new PublicKey(address);
    return true;
  } catch {
    return false;
  }
}
