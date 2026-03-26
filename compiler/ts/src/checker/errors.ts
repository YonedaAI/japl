import { Span } from '../lexer/token.js';

export class TypeError extends Error {
  constructor(
    public override message: string,
    public span: Span,
    public notes?: string[],
  ) {
    super(message);
    this.name = 'TypeError';
  }

  toString(): string {
    let msg = `TypeError at line ${this.span.line}, col ${this.span.col}: ${this.message}`;
    if (this.notes) {
      for (const note of this.notes) {
        msg += `\n  note: ${note}`;
      }
    }
    return msg;
  }
}
