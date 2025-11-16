export class BitQuanError extends Error {
  public code: string;
  public details?: any;

  constructor(code: string, message: string, details?: any) {
    super(message);
    this.name = 'BitQuanError';
    this.code = code;
    this.details = details;
  }

  static isBitQuanError(error: any): error is BitQuanError {
    return error instanceof BitQuanError;
  }
}

export function createBitQuanError(code: string, message: string, details?: any): BitQuanError {
  return new BitQuanError(code, message, details);
}