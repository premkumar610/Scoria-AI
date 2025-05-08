// client/src/lib/crypto.ts

interface KeyPair {
  publicKey: CryptoKey;
  privateKey: CryptoKey;
}

interface EncryptParams {
  algorithm: 'AES-GCM' | 'AES-CBC';
  plaintext: Uint8Array;
  key: CryptoKey;
  additionalData?: ArrayBuffer;
}

interface SignParams {
  algorithm: 'HMAC' | 'ECDSA';
  data: Uint8Array;
  key: CryptoKey;
}

interface HashParams {
  algorithm: 'SHA-256' | 'SHA-384' | 'SHA-512';
  data: Uint8Array;
}

export class CryptoError extends Error {
  constructor(
    public readonly code: string,
    message: string,
    public readonly cause?: unknown
  ) {
    super(message);
    this.name = 'CryptoError';
  }
}

export async function generateKey(
  algorithm: AlgorithmIdentifier,
  extractable = true,
  keyUsages: KeyUsage[] = ['encrypt', 'decrypt']
): Promise<KeyPair> {
  try {
    switch (algorithm.name) {
      case 'AES-GCM':
        const aesKey = await crypto.subtle.generateKey(
          {
            name: 'AES-GCM',
            length: 256,
          },
          extractable,
          keyUsages
        );
        return { publicKey: aesKey, privateKey: aesKey };

      case 'HMAC':
        const hmacKey = await crypto.subtle.generateKey(
          {
            name: 'HMAC',
            hash: 'SHA-256',
            length: 256,
          },
          extractable,
          ['sign', 'verify']
        );
        return { publicKey: hmacKey, privateKey: hmacKey };

      default:
        throw new CryptoError(
          'UNSUPPORTED_ALGORITHM',
          `Unsupported algorithm: ${algorithm}`
        );
    }
  } catch (error) {
    throw new CryptoError('KEY_GEN_FAILED', 'Key generation failed', error);
  }
}

export async function encrypt({
  algorithm,
  plaintext,
  key,
  additionalData,
}: EncryptParams): Promise<{ iv: Uint8Array; ciphertext: Uint8Array }> {
  try {
    const iv = crypto.getRandomValues(new Uint8Array(12));
    const ciphertext = await crypto.subtle.encrypt(
      {
        name: algorithm,
        iv,
        additionalData,
        tagLength: 128,
      },
      key,
      plaintext
    );

    return {
      iv,
      ciphertext: new Uint8Array(ciphertext),
    };
  } catch (error) {
    throw new CryptoError('ENCRYPT_FAILED', 'Encryption failed', error);
  }
}

export async function decrypt(
  algorithm: 'AES-GCM' | 'AES-CBC',
  ciphertext: Uint8Array,
  key: CryptoKey,
  iv: Uint8Array,
  additionalData?: ArrayBuffer
): Promise<Uint8Array> {
  try {
    const plaintext = await crypto.subtle.decrypt(
      {
        name: algorithm,
        iv,
        additionalData,
        tagLength: 128,
      },
      key,
      ciphertext
    );

    return new Uint8Array(plaintext);
  } catch (error) {
    throw new CryptoError('DECRYPT_FAILED', 'Decryption failed', error);
  }
}

export async function sign({ algorithm, data, key }: SignParams): Promise<Uint8Array> {
  try {
    let signAlgorithm;
    switch (algorithm) {
      case 'HMAC':
        signAlgorithm = { name: 'HMAC' };
        break;
      case 'ECDSA':
        signAlgorithm = {
          name: 'ECDSA',
          hash: 'SHA-256',
        };
        break;
      default:
        throw new CryptoError(
          'UNSUPPORTED_ALGORITHM',
          `Unsupported signing algorithm: ${algorithm}`
        );
    }

    const signature = await crypto.subtle.sign(signAlgorithm, key, data);
    return new Uint8Array(signature);
  } catch (error) {
    throw new CryptoError('SIGN_FAILED', 'Signing failed', error);
  }
}

export async function verifySignature(
  { algorithm, data, key }: SignParams,
  signature: Uint8Array
): Promise<boolean> {
  try {
    let verifyAlgorithm;
    switch (algorithm) {
      case 'HMAC':
        verifyAlgorithm = { name: 'HMAC' };
        break;
      case 'ECDSA':
        verifyAlgorithm = {
          name: 'ECDSA',
          hash: 'SHA-256',
        };
        break;
      default:
        throw new CryptoError(
          'UNSUPPORTED_ALGORITHM',
          `Unsupported verification algorithm: ${algorithm}`
        );
    }

    return await crypto.subtle.verify(
      verifyAlgorithm,
      key,
      signature,
      data
    );
  } catch (error) {
    throw new CryptoError('VERIFY_FAILED', 'Verification failed', error);
  }
}

export async function deriveKey(
  algorithm: 'ECDH',
  publicKey: CryptoKey,
  privateKey: CryptoKey,
  derivedKeyAlgorithm: AesDerivedKeyParams
): Promise<CryptoKey> {
  try {
    return await crypto.subtle.deriveKey(
      {
        name: algorithm,
        public: publicKey,
      },
      privateKey,
      derivedKeyAlgorithm,
      true,
      ['encrypt', 'decrypt']
    );
  } catch (error) {
    throw new CryptoError('KEY_DERIVE_FAILED', 'Key derivation failed', error);
  }
}

export async function hash({ algorithm, data }: HashParams): Promise<Uint8Array> {
  try {
    const digest = await crypto.subtle.digest(algorithm, data);
    return new Uint8Array(digest);
  } catch (error) {
    throw new CryptoError('HASH_FAILED', 'Hashing failed', error);
  }
}

export async function exportKey(
  key: CryptoKey,
  format: 'raw' | 'jwk' = 'jwk'
): Promise<JsonWebKey | Uint8Array> {
  try {
    const exported = await crypto.subtle.exportKey(format, key);
    return format === 'jwk'
      ? (exported as JsonWebKey)
      : new Uint8Array(exported as ArrayBuffer);
  } catch (error) {
    throw new CryptoError('KEY_EXPORT_FAILED', 'Key export failed', error);
  }
}

export async function importKey(
  format: 'raw' | 'jwk',
  keyData: JsonWebKey | Uint8Array,
  algorithm: AlgorithmIdentifier,
  keyUsages: KeyUsage[]
): Promise<CryptoKey> {
  try {
    return await crypto.subtle.importKey(
      format,
      keyData,
      algorithm,
      true,
      keyUsages
    );
  } catch (error) {
    throw new CryptoError('KEY_IMPORT_FAILED', 'Key import failed', error);
  }
}

export function generateSalt(length = 16): Uint8Array {
  return crypto.getRandomValues(new Uint8Array(length));
}

export async function pbkdf2(
  password: Uint8Array,
  salt: Uint8Array,
  iterations = 100000,
  hash: 'SHA-256' | 'SHA-384' = 'SHA-256',
  length = 256
): Promise<CryptoKey> {
  try {
    const baseKey = await crypto.subtle.importKey(
      'raw',
      password,
      'PBKDF2',
      false,
      ['deriveKey']
    );

    return crypto.subtle.deriveKey(
      {
        name: 'PBKDF2',
        salt,
        iterations,
        hash,
      },
      baseKey,
      { name: 'AES-GCM', length },
      true,
      ['encrypt', 'decrypt']
    );
  } catch (error) {
    throw new CryptoError('PBKDF2_FAILED', 'PBKDF2 derivation failed', error);
  }
}
