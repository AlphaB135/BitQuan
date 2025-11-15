import { Transaction, TransactionInput, TransactionOutput, OutPoint } from './types';
import { BitQuanError } from './errors';
import { Wallet } from './wallet';

export class TransactionBuilder {
  private versionNum: number = 1;
  private inputs: TransactionInput[] = [];
  private outputs: TransactionOutput[] = [];
  private lockTimeVal: number = 0;
  private feeRateVal: number = 1000; // satoshis per byte

  version(version: number): TransactionBuilder {
    this.versionNum = version;
    return this;
  }

  addInput(previousTx: string, outputIndex: number): TransactionBuilder {
    const input: TransactionInput = {
      previous_output: {
        hash: previousTx,
        index: outputIndex
      },
      sequence: 0xFFFFFFFF
    };
    this.inputs.push(input);
    return this;
  }

  addOutput(address: string, amount: number): TransactionBuilder {
    const output: TransactionOutput = {
      value: amount,
      script_pubkey: this.addressToScriptPubkey(address),
      address
    };
    this.outputs.push(output);
    return this;
  }

  lockTime(lockTime: number): TransactionBuilder {
    this.lockTimeVal = lockTime;
    return this;
  }

  feeRate(feeRate: number): TransactionBuilder {
    this.feeRateVal = feeRate;
    return this;
  }

  build(): Transaction {
    if (this.inputs.length === 0) {
      throw new BitQuanError('INVALID_TRANSACTION', 'Transaction must have at least one input');
    }

    if (this.outputs.length === 0) {
      throw new BitQuanError('INVALID_TRANSACTION', 'Transaction must have at least one output');
    }

    const txSize = this.estimateSize();
    const fee = txSize * this.feeRateVal;

    return {
      version: this.versionNum,
      inputs: this.inputs,
      outputs: this.outputs,
      lock_time: this.lockTimeVal,
      fee,
      timestamp: Date.now()
    };
  }

  private estimateSize(): number {
    const baseSize = 10; // version + lock_time + input/output counts
    const inputSize = this.inputs.length * 40; // Approximate input size
    const outputSize = this.outputs.length * 34; // Approximate output size
    
    return baseSize + inputSize + outputSize;
  }

  private addressToScriptPubkey(address: string): string {
    // Simplified script pubkey generation
    return `OP_DUP OP_HASH160 ${address} OP_EQUALVERIFY OP_CHECKSIG`;
  }
}

export class TransactionUtils {
  static hash(transaction: Transaction): string {
    const serialized = this.serialize(transaction);
    return this.hashData(serialized);
  }

  static size(transaction: Transaction): number {
    return this.serialize(transaction).length;
  }

  static inputValue(transaction: Transaction): number {
    // In a real implementation, this would look up UTXO values
    return 0;
  }

  static outputValue(transaction: Transaction): number {
    return transaction.outputs.reduce((sum, output) => sum + output.value, 0);
  }

  static fee(transaction: Transaction): number {
    return transaction.fee;
  }

  static isValid(transaction: Transaction): boolean {
    return transaction.inputs.length > 0 && 
           transaction.outputs.length > 0 && 
           this.outputValue(transaction) > 0;
  }

  static serialize(transaction: Transaction): string {
    return JSON.stringify({
      version: transaction.version,
      inputs: transaction.inputs,
      outputs: transaction.outputs,
      lock_time: transaction.lock_time
    });
  }

  static deserialize(hex: string): Transaction {
    try {
      const data = JSON.parse(hex);
      return {
        version: data.version,
        inputs: data.inputs,
        outputs: data.outputs,
        lock_time: data.lock_time,
        fee: data.fee || 0,
        signature: data.signature,
        timestamp: data.timestamp
      };
    } catch (error) {
      throw new BitQuanError('DESERIALIZATION_ERROR', `Failed to deserialize transaction: ${error}`);
    }
  }

  static sign(transaction: Transaction, privateKey: string): Transaction {
    const signature = this.signTransaction(transaction, privateKey);
    return {
      ...transaction,
      signature
    };
  }

  static verifySignature(transaction: Transaction): boolean {
    return transaction.signature !== undefined;
  }

  private static hashData(data: string): string {
    // Simplified hashing - in production, use proper cryptographic hash functions
    let hash = 0;
    for (let i = 0; i < data.length; i++) {
      const char = data.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash;
    }
    return Math.abs(hash).toString(16);
  }

  private static signTransaction(transaction: Transaction, privateKey: string): string {
    const serialized = this.serialize(transaction);
    const hash = this.hashData(serialized);
    return `signed_${privateKey.substring(0, 8)}_${hash}`;
  }
}

export class MerkleTree {
  private root: string;
  private leaves: string[];
  private depth: number;

  constructor(leaves: string[]) {
    this.leaves = [...leaves];
    this.depth = 0;
    this.root = this.build();
  }

  getRoot(): string {
    return this.root;
  }

  getProof(leafIndex: number): string[] {
    if (leafIndex >= this.leaves.length) {
      return [];
    }

    const proof: string[] = [];
    let currentLevel = [...this.leaves];
    let index = leafIndex;

    while (currentLevel.length > 1) {
      const isRight = index % 2 === 1;
      const siblingIndex = isRight ? index - 1 : index + 1;
      
      if (siblingIndex < currentLevel.length) {
        proof.push(currentLevel[siblingIndex]);
      } else {
        proof.push(currentLevel[index]);
      }

      const nextLevel: string[] = [];
      for (let i = 0; i < currentLevel.length; i += 2) {
        const left = currentLevel[i];
        const right = i + 1 < currentLevel.length ? currentLevel[i + 1] : left;
        const combined = left + right;
        nextLevel.push(this.hash(combined));
      }
      
      currentLevel = nextLevel;
      index = Math.floor(index / 2);
    }

    return proof;
  }

  static verifyProof(leaf: string, proof: string[], root: string): boolean {
    let currentHash = leaf;
    
    for (const sibling of proof) {
      const combined = currentHash + sibling;
      currentHash = MerkleTree.prototype.hash(combined);
    }
    
    return currentHash === root;
  }

  private build(): string {
    if (this.leaves.length === 0) {
      return '';
    }

    let currentLevel = [...this.leaves];
    this.depth = 0;

    while (currentLevel.length > 1) {
      const nextLevel: string[] = [];
      
      for (let i = 0; i < currentLevel.length; i += 2) {
        const left = currentLevel[i];
        const right = i + 1 < currentLevel.length ? currentLevel[i + 1] : left;
        const combined = left + right;
        nextLevel.push(this.hash(combined));
      }
      
      currentLevel = nextLevel;
      this.depth++;
    }

    return currentLevel[0];
  }

  private hash(data: string): string {
    // Simplified hashing - in production, use proper cryptographic hash functions
    let hash = 0;
    for (let i = 0; i < data.length; i++) {
      const char = data.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash;
    }
    return Math.abs(hash).toString(16);
  }
}