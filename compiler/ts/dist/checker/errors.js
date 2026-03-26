export class TypeError extends Error {
    message;
    span;
    notes;
    constructor(message, span, notes) {
        super(message);
        this.message = message;
        this.span = span;
        this.notes = notes;
        this.name = 'TypeError';
    }
    toString() {
        let msg = `TypeError at line ${this.span.line}, col ${this.span.col}: ${this.message}`;
        if (this.notes) {
            for (const note of this.notes) {
                msg += `\n  note: ${note}`;
            }
        }
        return msg;
    }
}
//# sourceMappingURL=errors.js.map