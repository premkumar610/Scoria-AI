// client/src/lib/client.ts

import axios, { 
  AxiosInstance, 
  AxiosRequestConfig, 
  AxiosResponse,
  AxiosError 
} from 'axios';
import { Connection, Keypair, Transaction } from '@solana/web3.js';
import { sign } from 'tweetnacl';

interface ScoriaClientConfig {
  baseURL: string;
  rpcEndpoint: string;
  timeout?: number;
  authToken?: string;
  signer?: Keypair;
}

enum HttpStatusCode {
  TooManyRequests = 429,
  Unauthorized = 401,
}

export class ScoriaHttpClient {
  private instance: AxiosInstance;
  private connection: Connection;
  private signer?: Keypair;

  constructor(config: ScoriaClientConfig) {
    this.connection = new Connection(config.rpcEndpoint);
    this.signer = config.signer;

    this.instance = axios.create({
      baseURL: config.baseURL,
      timeout: config.timeout || 30_000,
      headers: {
        'Content-Type': 'application/json',
        'X-Client': 'ScoriaAI/2.0',
      },
    });

    this._initializeInterceptors();
  }

  private _initializeInterceptors(): void {
    // Request interceptor
    this.instance.interceptors.request.use(
      async (config) => {
        if (this.signer) {
          const { signature, timestamp } = await this._signRequest(config);
          config.headers['X-Signature'] = signature;
          config.headers['X-Timestamp'] = timestamp;
          config.headers['X-PublicKey'] = this.signer.publicKey.toBase58();
        }
        
        if (this.instance.defaults.headers.common['Authorization']) {
          config.headers['Authorization'] = 
            this.instance.defaults.headers.common['Authorization'];
        }

        return config;
      },
      (error) => Promise.reject(error)
    );

    // Response interceptor
    this.instance.interceptors.response.use(
      (response) => this._handleResponse(response),
      (error) => this._handleError(error)
    );
  }

  private async _signRequest(config: AxiosRequestConfig): Promise<{
    signature: string;
    timestamp: number;
  }> {
    const timestamp = Date.now();
    const signData = {
      method: config.method?.toUpperCase(),
      path: config.url,
      body: config.data || {},
      timestamp,
    };

    const message = new TextEncoder().encode(JSON.stringify(signData));
    const signature = sign(message, this.signer!.secretKey);
    
    return {
      signature: Buffer.from(signature).toString('base64'),
      timestamp,
    };
  }

  private _handleResponse(response: AxiosResponse) {
    if (response.data?.transaction && this.signer) {
      const tx = Transaction.from(Buffer.from(response.data.transaction, 'base64'));
      return this._signAndSendTransaction(tx);
    }
    return response.data;
  }

  private async _signAndSendTransaction(tx: Transaction): Promise<string> {
    if (!this.signer) throw new Error('Signer not initialized');
    
    tx.recentBlockhash = (await this.connection.getLatestBlockhash()).blockhash;
    tx.sign(this.signer);
    
    const rawTx = tx.serialize();
    return this.connection.sendRawTransaction(rawTx);
  }

  private _handleError(error: AxiosError) {
    if (error.response) {
      const { status, data } = error.response;
      
      switch (status) {
        case HttpStatusCode.Unauthorized:
          this._refreshAuthToken();
          break;
          
        case HttpStatusCode.TooManyRequests:
          return this._handleRateLimit(data);
      }
      
      throw new Error(`API Error ${status}: ${JSON.stringify(data)}`);
    }
    
    throw error;
  }

  private async _refreshAuthToken(): Promise<void> {
    // Implement OAuth2 refresh flow
  }

  private _handleRateLimit(data: any): Promise<void> {
    const retryAfter = data?.retryAfter || 5;
    return new Promise(resolve => 
      setTimeout(resolve, retryAfter * 1000)
    );
  }

  // Core API Methods
  public async get<T>(url: string, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.get<T>(url, config);
  }

  public async post<T>(url: string, data?: any, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.post<T>(url, data, config);
  }

  public async put<T>(url: string, data?: any, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.put<T>(url, data, config);
  }

  public async delete<T>(url: string, config?: AxiosRequestConfig): Promise<T> {
    return this.instance.delete<T>(url, config);
  }

  // Web3 Specific Methods
  public async submitZKProof(proof: string): Promise<string> {
    const response = await this.post('/zk/submit', { proof });
    return response.txSignature;
  }

  public async fetchModelMetadata(modelHash: string): Promise<any> {
    return this.get(`/models/${encodeURIComponent(modelHash)}/metadata`);
  }
}

// Usage Example
const client = new ScoriaHttpClient({
  baseURL: 'https://api.scoria.ai/v1',
  rpcEndpoint: 'https://solana-mainnet.scoria.ai',
  signer: Keypair.fromSecretKey(/* ... */),
});

// Execute signed API request
const modelMetadata = await client.fetchModelMetadata('bafybeiemxf5abjwjbikoz4mc3...');
