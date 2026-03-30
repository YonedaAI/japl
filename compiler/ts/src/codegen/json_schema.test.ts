import { describe, it, expect } from 'vitest';
import { typeToJsonSchema, typeExprToSchema } from './json_schema.js';
import type { Decl, TypeExpr } from '../parser/ast.js';
import { Lexer } from '../lexer/index.js';
import { Parser } from '../parser/index.js';

const dummySpan = { start: 0, end: 0, line: 1, col: 1 };

// ─── Helper: build AST nodes without parsing ───

function mkNamed(name: string, args: TypeExpr[] = []): TypeExpr {
  return { kind: 'tnamed', name, args, span: dummySpan };
}

function mkSumType(name: string, variants: { name: string; fields: TypeExpr[] }[]): Decl {
  return {
    kind: 'type',
    name,
    typeParams: [],
    variants: variants.map(v => ({ name: v.name, fields: v.fields, span: dummySpan })),
    span: dummySpan,
  };
}

function mkRecordType(name: string, fields: { name: string; type: TypeExpr }[]): Decl {
  return {
    kind: 'record_type',
    name,
    typeParams: [],
    fields: fields.map(f => ({ name: f.name, type: f.type, span: dummySpan })),
    span: dummySpan,
  };
}

// ─── Tests ───

describe('typeExprToSchema — primitive mapping', () => {
  it('maps Int to integer', () => {
    expect(typeExprToSchema(mkNamed('Int'))).toEqual({ type: 'integer' });
  });

  it('maps Float to number', () => {
    expect(typeExprToSchema(mkNamed('Float'))).toEqual({ type: 'number' });
  });

  it('maps String to string', () => {
    expect(typeExprToSchema(mkNamed('String'))).toEqual({ type: 'string' });
  });

  it('maps Bool to boolean', () => {
    expect(typeExprToSchema(mkNamed('Bool'))).toEqual({ type: 'boolean' });
  });
});

describe('typeExprToSchema — compound types', () => {
  it('maps List(Int) to array of integers', () => {
    const schema = typeExprToSchema(mkNamed('List', [mkNamed('Int')])) as any;
    expect(schema.type).toBe('array');
    expect(schema.items).toEqual({ type: 'integer' });
  });

  it('maps Option(String) to oneOf with null', () => {
    const schema = typeExprToSchema(mkNamed('Option', [mkNamed('String')])) as any;
    expect(schema.oneOf).toEqual([{ type: 'string' }, { type: 'null' }]);
  });

  it('maps tuple to fixed-length array', () => {
    const te: TypeExpr = {
      kind: 'ttuple',
      elements: [mkNamed('Int'), mkNamed('String')],
      span: dummySpan,
    };
    const schema = typeExprToSchema(te) as any;
    expect(schema.type).toBe('array');
    expect(schema.items).toEqual([{ type: 'integer' }, { type: 'string' }]);
    expect(schema.minItems).toBe(2);
    expect(schema.maxItems).toBe(2);
  });

  it('maps unit to null', () => {
    const te: TypeExpr = { kind: 'tunit', span: dummySpan };
    expect(typeExprToSchema(te)).toEqual({ type: 'null' });
  });

  it('maps tvar to empty schema (any)', () => {
    const te: TypeExpr = { kind: 'tvar', name: 'a', span: dummySpan };
    expect(typeExprToSchema(te)).toEqual({});
  });
});

describe('typeToJsonSchema — sum types', () => {
  it('generates schema for Sentiment sum type', () => {
    const decl = mkSumType('Sentiment', [
      { name: 'Positive', fields: [mkNamed('Float')] },
      { name: 'Negative', fields: [mkNamed('Float')] },
      { name: 'Neutral', fields: [] },
    ]);
    const schema = typeToJsonSchema(decl) as any;

    expect(schema.oneOf).toHaveLength(3);

    // Positive(Float)
    expect(schema.oneOf[0].properties._tag.const).toBe('Positive');
    expect(schema.oneOf[0].properties._0).toEqual({ type: 'number' });
    expect(schema.oneOf[0].required).toEqual(['_tag', '_0']);
    expect(schema.oneOf[0].additionalProperties).toBe(false);

    // Neutral — no payload fields
    expect(schema.oneOf[2].properties._tag.const).toBe('Neutral');
    expect(schema.oneOf[2].required).toEqual(['_tag']);
  });

  it('generates schema for multi-field variant', () => {
    const decl = mkSumType('Shape', [
      { name: 'Circle', fields: [mkNamed('Float')] },
      { name: 'Rect', fields: [mkNamed('Float'), mkNamed('Float')] },
    ]);
    const schema = typeToJsonSchema(decl) as any;

    expect(schema.oneOf).toHaveLength(2);
    const rect = schema.oneOf[1];
    expect(rect.properties._tag.const).toBe('Rect');
    expect(rect.properties._0).toEqual({ type: 'number' });
    expect(rect.properties._1).toEqual({ type: 'number' });
    expect(rect.required).toEqual(['_tag', '_0', '_1']);
  });
});

describe('typeToJsonSchema — record types', () => {
  it('generates schema for User record', () => {
    const decl = mkRecordType('User', [
      { name: 'name', type: mkNamed('String') },
      { name: 'age', type: mkNamed('Int') },
      { name: 'active', type: mkNamed('Bool') },
    ]);
    const schema = typeToJsonSchema(decl) as any;

    expect(schema.type).toBe('object');
    expect(schema.properties.name).toEqual({ type: 'string' });
    expect(schema.properties.age).toEqual({ type: 'integer' });
    expect(schema.properties.active).toEqual({ type: 'boolean' });
    expect(schema.required).toEqual(['name', 'age', 'active']);
    expect(schema.additionalProperties).toBe(false);
  });

  it('generates schema for record with nested List', () => {
    const decl = mkRecordType('Team', [
      { name: 'members', type: mkNamed('List', [mkNamed('String')]) },
      { name: 'score', type: mkNamed('Float') },
    ]);
    const schema = typeToJsonSchema(decl) as any;

    expect(schema.properties.members).toEqual({ type: 'array', items: { type: 'string' } });
    expect(schema.properties.score).toEqual({ type: 'number' });
  });
});

describe('typeToJsonSchema — unsupported decl kinds', () => {
  it('returns empty object for fn decl', () => {
    const decl: Decl = {
      kind: 'fn',
      name: 'foo',
      params: [],
      body: { kind: 'unit', span: dummySpan },
      pub: false,
      span: dummySpan,
    };
    expect(typeToJsonSchema(decl)).toEqual({});
  });
});

describe('integration — schema from parsed JAPL source', () => {
  function parseFirstDecl(source: string): Decl {
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    const ast = parser.parse();
    const errors = parser.getErrors();
    if (errors.length > 0) {
      throw new Error(`Parse errors: ${errors.map(e => e.message).join(', ')}`);
    }
    return ast.decls[0];
  }

  it('generates schema from parsed sum type source', () => {
    const decl = parseFirstDecl(`
type Color =
  | Red
  | Green
  | Blue
`);
    const schema = typeToJsonSchema(decl) as any;
    expect(schema.oneOf).toHaveLength(3);
    expect(schema.oneOf[0].properties._tag.const).toBe('Red');
    expect(schema.oneOf[1].properties._tag.const).toBe('Green');
    expect(schema.oneOf[2].properties._tag.const).toBe('Blue');
  });
});
