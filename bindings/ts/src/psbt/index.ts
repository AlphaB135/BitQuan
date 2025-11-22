/**
 * Post-Quantum PSBT (PQ-PSBT) implementation for BitQuan
 */

import { Address, Network } from '../address';
import { Transaction, SignatureAlgorithm, PSBTInput, PSBTOutput, ValidationResult } from '../types';

/**
 * PSBT errors
 */
export class PSBTError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'PSBTError';
  }
}

/**
 * Signature algorithm flags
 */
export class SignatureFlags {
  private flags: number;

  constructor(flags: number = 0) {
    this.flags = flags;
  }

  /**
   * Create new flags
   */
  static new(): SignatureFlags {
    return new SignatureFlags(0);
  }

  /**
   * Set Dilithium signature flag
   */
  withDilithium(): SignatureFlags {
    return new SignatureFlags(this.flags | 0x01);
  }

  /**
   * Set ECDSA fallback flag
   */
  withECDSA(): SignatureFlags {
    return new SignatureFlags(this.flags | 0x02);
  }

  /**
   * Set hybrid mode (both signatures required)
   */
  withHybrid(): SignatureFlags {
    return new SignatureFlags(this.flags | 0x04);
  }

  /**
   * Check if Dilithium signature is present
   */
  hasDilithium(): boolean {
    return (this.flags & 0x01) !== 0;
  }

  /**
   * Check if ECDSA fallback is present
   */
  hasECDSA(): boolean {
    return (this.flags & 0x02) !== 0;
  }

  /**
   * Check if hybrid mode is required
   */
  isHybrid(): boolean {
    return (this.flags & 0x04) !== 0;
  }

  /**
   * Get flag value
   */
  value(): number {
    return this.flags;
  }

  /**
   * Convert to buffer
   */
  toBuffer(): Buffer {
    return Buffer.from([this.flags]);
  }

  /**
   * Create from buffer
   */
  static fromBuffer(buffer: Buffer): SignatureFlags {
    return new SignatureFlags(buffer[0]);
  }
}

/**
 * PQ-PSBT magic bytes and constants
 */
const PQ_PSBT_MAGIC = Buffer.from('PQPS', 'ascii');
const PQ_PSBT_VERSION = 0x00;

/**
 * Global PSBT keys
 */
export enum GlobalKey {
  Version = 0x01,
  FallbackFingerprint = 0x02,
  Locktime = 0x03,
  Proprietary = 0x80
}

/**
 * Input PSBT keys
 */
export enum InputKey {
  PreviousTxid = 0x01,
  PreviousOutputIndex = 0x02,
  Sequence = 0x03,
  ScriptSig = 0x04,
  DilithiumPublicKey = 0x05,
  DilithiumSignature = 0x06,
  ECDSASignature = 0x07,
  Proprietary = 0x80
}

/**
 * Output PSBT keys
 */
export enum OutputKey {
  Amount = 0x01,
  ScriptPubkey = 0x02,
  Proprietary = 0x80
}

/**
 * Post-Quantum PSBT implementation
 */
export class PQPSBT {
  public readonly version: number;
  public readonly signatureFlags: SignatureFlags;
  public readonly global: Map<GlobalKey, Buffer>;
  public readonly inputs: PSBTInput[];
  public readonly outputs: PSBTOutput[];

  constructor(
    version: number = PQ_PSBT_VERSION,
    signatureFlags: SignatureFlags = SignatureFlags.new().withDilithium(),
    global: Map<GlobalKey, Buffer> = new Map(),
    inputs: PSBTInput[] = [],
    outputs: PSBTOutput[] = []
  ) {
    this.version = version;
    this.signatureFlags = signatureFlags;
    this.global = global;
    this.inputs = inputs;
    this.outputs = outputs;
  }

  /**
   * Create PSBT builder
   */
  static builder(): PQPSBTBuilder {
    return new PQPSBTBuilder();
  }

  /**
   * Serialize PSBT to bytes
   */
  serialize(): Buffer {
    const buffers: Buffer[] = [];

    // Write magic
    buffers.push(PQ_PSBT_MAGIC);

    // Write version
    buffers.push(Buffer.from([this.version]));

    // Write flags
    buffers.push(this.signatureFlags.toBuffer());

    // Write global data
    buffers.push(this.serializeMap(this.global));

    // Write input count
    buffers.push(this.writeCompactSize(this.inputs.length));

    // Write inputs
    for (const input of this.inputs) {
      buffers.push(this.serializeInput(input));
    }

    // Write output count
    buffers.push(this.writeCompactSize(this.outputs.length));

    // Write outputs
    for (const output of this.outputs) {
      buffers.push(this.serializeOutput(output);
    }

    return Buffer.concat(buffers);
  }

  /**
   * Deserialize PSBT from bytes
   */
  static deserialize(data: Buffer): PQPSBT {
    if (data.length < 6) {
      throw new PSBTError('PSBT data too short');
    }

    // Check magic
    if (!data.subarray(0, 4).equals(PQ_PSBT_MAGIC)) {
      throw new PSBTError('Invalid PSBT magic');
    }

    let offset = 4;
    const version = data[offset++];
    const signatureFlags = SignatureFlags.fromBuffer(Buffer.from([data[offset++]]));

    // Read global data
    const [global, newOffset] = this.deserializeMap(data, offset);
    offset = newOffset;

    // Read input count
    const [inputCount, offset2] = this.readCompactSize(data, offset);
    offset = offset2;

    // Read inputs
    const inputs: PSBTInput[] = [];
    for (let i = 0; i < inputCount; i++) {
      const [input, newOffset] = this.deserializeInput(data, offset);
      inputs.push(input);
      offset = newOffset;
    }

    // Read output count
    const [outputCount, offset3] = this.readCompactSize(data, offset);
    offset = offset3;

    // Read outputs
    const outputs: PSBTOutput[] = [];
    for (let i = 0; i < outputCount; i++) {
      const [output, newOffset] = this.deserializeOutput(data, offset);
      outputs.push(output);
      offset = newOffset;
    }

    return new PQPSBT(version, signatureFlags, global, inputs, outputs);
  }

  /**
   * Finalize PSBT and extract transaction
   */
  finalize(): Transaction {
    // This would build the final transaction from PSBT data
    // Implementation depends on Transaction structure
    throw new PSBTError('PSBT finalization not yet implemented');
  }

  /**
   * Add input to PSBT
   */
  addInput(input: PSBTInput): void {
    this.inputs.push(input);
  }

  /**
   * Add output to PSBT
   */
  addOutput(output: PSBTOutput): void {
    this.outputs.push(output);
  }

  /**
   * Get input at index
   */
  getInput(index: number): PSBTInput | undefined {
    return this.inputs[index];
  }

  /**
   * Get output at index
   */
  getOutput(index: number): PSBTOutput | undefined {
    return this.outputs[index];
  }

  // Private helper methods

  private serializeMap(map: Map<any, Buffer>): Buffer {
    const buffers: Buffer[] = [];
    buffers.push(this.writeCompactSize(map.size));

    for (const [key, value] of map.entries()) {
      const keyBuffer = this.serializeKey(key);
      buffers.push(this.writeCompactSize(keyBuffer.length));
      buffers.push(keyBuffer);
      buffers.push(this.writeCompactSize(value.length));
      buffers.push(value);
    }

    return Buffer.concat(buffers);
  }

  private serializeKey(key: any): Buffer {
    // Simplified key serialization
    if (typeof key === 'number') {
      return Buffer.from([key]);
    }
    throw new PSBTError(`Unsupported key type: ${typeof key}`);
  }

  private serializeInput(input: PSBTInput): Buffer {
    const map = new Map();

    if (input.previousTxid) {
      map.set(InputKey.PreviousTxid, input.previousTxid);
    }
    if (input.previousOutputIndex !== undefined) {
      map.set(InputKey.PreviousOutputIndex, Buffer.from([input.previousOutputIndex]));
    }
    if (input.sequence !== undefined) {
      map.set(InputKey.Sequence, Buffer.from([input.sequence]));
    }
    if (input.scriptSig) {
      map.set(InputKey.ScriptSig, input.scriptSig);
    }
    if (input.dilithiumPublicKey) {
      map.set(InputKey.DilithiumPublicKey, input.dilithiumPublicKey);
    }
    if (input.dilithiumSignature) {
      map.set(InputKey.DilithiumSignature, input.dilithiumSignature);
    }
    if (input.ecdsaSignature) {
      map.set(InputKey.ECDSASignature, input.ecdsaSignature);
    }

    return this.serializeMap(map);
  }

  private serializeOutput(output: PSBTOutput): Buffer {
    const map = new Map();

    if (output.amount !== undefined) {
      const amountBuffer = Buffer.alloc(8);
      amountBuffer.writeBigUInt64BE(BigInt(output.amount), 0);
      map.set(OutputKey.Amount, amountBuffer);
    }
    if (output.scriptPubkey) {
      map.set(OutputKey.ScriptPubkey, output.scriptPubkey);
    }

    return this.serializeMap(map);
  }

  private static deserializeMap(data: Buffer, offset: number): [Map<any, Buffer>, number] {
    const [count, newOffset] = PQPSBT.prototype.readCompactSize(data, offset);
    let currentOffset = newOffset;

    const map = new Map();
    for (let i = 0; i < count; i++) {
      const [keyLen, offset1] = PQPSBT.prototype.readCompactSize(data, currentOffset);
      currentOffset = offset1;

      const key = data.subarray(currentOffset, currentOffset + keyLen);
      currentOffset += keyLen;

      const [valueLen, offset2] = PQPSBT.prototype.readCompactSize(data, currentOffset);
      currentOffset = offset2;

      const value = data.subarray(currentOffset, currentOffset + valueLen);
      currentOffset += valueLen;

      // Simplified key deserialization
      if (key.length === 1) {
        map.set(key[0], value);
      }
    }

    return [map, currentOffset];
  }

  private static deserializeInput(data: Buffer, offset: number): [PSBTInput, number] {
    const [map, newOffset] = PQPSBT.prototype.deserializeMap(data, offset);

    const input: PSBTInput = {
      previousTxid: map.get(InputKey.PreviousTxid) || Buffer.alloc(32),
      previousOutputIndex: map.get(InputKey.PreviousOutputIndex)?.[0] || 0,
      sequence: map.get(InputKey.Sequence)?.[0] || 0xffffffff,
      scriptSig: map.get(InputKey.ScriptSig) || Buffer.alloc(0),
      dilithiumPublicKey: map.get(InputKey.DilithiumPublicKey),
      dilithiumSignature: map.get(InputKey.DilithiumSignature),
      ecdsaSignature: map.get(InputKey.ECDSASignature)
    };

    return [input, newOffset];
  }

  private static deserializeOutput(data: Buffer, offset: number): [PSBTOutput, number] {
    const [map, newOffset] = PQPSBT.prototype.deserializeMap(data, offset);

    const output: PSBTOutput = {
      amount: 0,
      scriptPubkey: map.get(OutputKey.ScriptPubkey) || Buffer.alloc(0)
    };

    const amountBuffer = map.get(OutputKey.Amount);
    if (amountBuffer && amountBuffer.length === 8) {
      output.amount = Number(amountBuffer.readBigUInt64BE(0));
    }

    return [output, newOffset];
  }

  private writeCompactSize(size: number): Buffer {
    if (size < 0xfd) {
      return Buffer.from([size]);
    } else if (size <= 0xffff) {
      return Buffer.concat([Buffer.from([0xfd]), Buffer.from([size & 0xff, (size >> 8) & 0xff])]);
    } else if (size <= 0xffffffff) {
      return Buffer.concat([
        Buffer.from([0xfe]),
        Buffer.from([
          size & 0xff,
          (size >> 8) & 0xff,
          (size >> 16) & 0xff,
          (size >> 24) & 0xff
        ])
      ]);
    } else {
      return Buffer.concat([
        Buffer.from([0xff]),
        Buffer.from([
          size & 0xff,
          (size >> 8) & 0xff,
          (size >> 16) & 0xff,
          (size >> 24) & 0xff,
          (size >> 32) & 0xff,
          (size >> 40) & 0xff,
          (size >> 48) & 0xff,
          (size >> 56) & 0xff
        ])
      ]);
    }
  }

  private readCompactSize(data: Buffer, offset: number): [number, number] {
    const first = data[offset];

    if (first < 0xfd) {
      return [first, offset + 1];
    } else if (first === 0xfd) {
      return [data[offset + 1] | (data[offset + 2] << 8), offset + 3];
    } else if (first === 0xfe) {
      return [
        data[offset + 1] |
        (data[offset + 2] << 8) |
        (data[offset + 3] << 16) |
        (data[offset + 4] << 24),
        offset + 5
      ];
    } else {
      return [
        Number(
          (data[offset + 1] |
          (data[offset + 2] << 8) |
          (data[offset + 3] << 16) |
          (data[offset + 4] << 24) |
          (BigInt(data[offset + 5]) << 32n) |
          (BigInt(data[offset + 6]) << 40n) |
          (BigInt(data[offset + 7]) << 48n) |
          (BigInt(data[offset + 8]) << 56n))
        ),
        offset + 9
      ];
    }
  }
}

/**
 * PSBT builder for convenient construction
 */
export class PQPSBTBuilder {
  private psbt: PQPSBT;

  constructor() {
    this.psbt = new PQPSBT();
  }

  /**
   * Set transaction version
   */
  version(version: number): PQPSBTBuilder {
    this.psbt.global.set(GlobalKey.Version, Buffer.from([version]));
    return this;
  }

  /**
   * Set locktime
   */
  locktime(locktime: number): PQPSBTBuilder {
    const buffer = Buffer.alloc(4);
    buffer.writeUInt32LE(locktime, 0);
    this.psbt.global.set(GlobalKey.Locktime, buffer);
    return this;
  }

  /**
   * Set signature flags
   */
  signatureFlags(flags: SignatureFlags): PQPSBTBuilder {
    this.psbt.signatureFlags = flags;
    return this;
  }

  /**
   * Add input
   */
  addInput(txid: string, vout: number): PQPSBTBuilder {
    const txidBuffer = Buffer.from(txid, 'hex').reverse();

    const input: PSBTInput = {
      previousTxid: txidBuffer,
      previousOutputIndex: vout,
      sequence: 0xffffffff,
      scriptSig: Buffer.alloc(0)
    };

    this.psbt.addInput(input);
    return this;
  }

  /**
   * Add output
   */
  addOutput(address: string, amount: number): PQPSBTBuilder {
    const addr = Address.fromString(address);
    const scriptPubkey = this.buildScriptPubkey(addr);

    const output: PSBTOutput = {
      amount,
      scriptPubkey
    };

    this.psbt.addOutput(output);
    return this;
  }

  /**
   * Build PSBT
   */
  build(): PQPSBT {
    return this.psbt;
  }

  /**
   * Build script pubkey from address
   */
  private buildScriptPubkey(address: Address): Buffer {
    switch (address.addressType) {
      case 0x00: // P2PKH
        return Buffer.concat([
          Buffer.from([0x76, 0xa9, 0x14]), // OP_DUP OP_HASH160 OP_DATA_20
          address.data,
          Buffer.from([0x88, 0xac]) // OP_EQUALVERIFY OP_CHECKSIG
        ]);

      case 0x02: // P2WPKH
        return Buffer.concat([
          Buffer.from([0x00, 0x14]), // OP_0 OP_DATA_20
          address.data
        ]);

      case 0x10: // PQ-P2PKH
        return Buffer.concat([
          Buffer.from([0x76, 0xa9, 0x14]), // OP_DUP OP_HASH160 OP_DATA_20
          address.data,
          Buffer.from([0x88, 0xac]) // OP_EQUALVERIFY OP_CHECKSIG
        ]);

      default:
        throw new PSBTError(`Unsupported address type: ${address.addressType}`);
    }
  }
}

// Export types
export type { PSBTError };
