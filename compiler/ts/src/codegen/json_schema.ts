// ─── JSON Schema Generator ───
// Generates JSON schemas from JAPL type declarations.
// Used for structured LLM output — the schema constrains the LLM's
// response format so it can be parsed back into typed JAPL values.

import type { Decl, TypeExpr, Variant, Field } from '../parser/ast.js';

/** Convert a JAPL type declaration to a JSON Schema object. */
export function typeToJsonSchema(decl: Decl): object {
  if (decl.kind === 'type') {
    return sumTypeToSchema(decl.name, decl.variants);
  }

  if (decl.kind === 'record_type') {
    return recordTypeToSchema(decl.fields);
  }

  return {};
}

/** Sum type (tagged union) → oneOf with _tag discriminator. */
function sumTypeToSchema(name: string, variants: Variant[]): object {
  return {
    oneOf: variants.map(variantToSchema),
  };
}

function variantToSchema(v: Variant): object {
  const properties: Record<string, object> = {
    _tag: { type: 'string' as const, const: v.name },
  };
  const required: string[] = ['_tag'];

  for (let i = 0; i < v.fields.length; i++) {
    const key = `_${i}`;
    properties[key] = typeExprToSchema(v.fields[i]);
    required.push(key);
  }

  return {
    type: 'object',
    properties,
    required,
    additionalProperties: false,
  };
}

/** Record type → object schema with named fields. */
function recordTypeToSchema(fields: Field[]): object {
  const properties: Record<string, object> = {};
  const required: string[] = [];

  for (const f of fields) {
    properties[f.name] = typeExprToSchema(f.type);
    required.push(f.name);
  }

  return {
    type: 'object',
    properties,
    required,
    additionalProperties: false,
  };
}

/** Map a JAPL type expression to the corresponding JSON Schema. */
export function typeExprToSchema(te: TypeExpr): object {
  switch (te.kind) {
    case 'tnamed':
      return namedTypeToSchema(te.name, te.args);
    case 'ttuple':
      return {
        type: 'array',
        items: te.elements.map(typeExprToSchema),
        minItems: te.elements.length,
        maxItems: te.elements.length,
      };
    case 'trecord':
      return recordExprToSchema(te.fields);
    case 'tunit':
      return { type: 'null' };
    case 'tfn':
      // Functions can't be serialised to JSON — return opaque string.
      return { type: 'string' };
    case 'tvar':
      // Unconstrained type variable — accept anything.
      return {};
    default:
      return { type: 'string' };
  }
}

function namedTypeToSchema(name: string, args: TypeExpr[]): object {
  switch (name) {
    case 'Int':
      return { type: 'integer' };
    case 'Float':
      return { type: 'number' };
    case 'String':
      return { type: 'string' };
    case 'Bool':
      return { type: 'boolean' };
    case 'List': {
      const itemSchema = args.length > 0 ? typeExprToSchema(args[0]) : {};
      return { type: 'array', items: itemSchema };
    }
    case 'Option': {
      const innerSchema = args.length > 0 ? typeExprToSchema(args[0]) : {};
      return { oneOf: [innerSchema, { type: 'null' }] };
    }
    default:
      // Unknown named type — fall back to string.
      return { type: 'string' };
  }
}

function recordExprToSchema(fields: [string, TypeExpr][]): object {
  const properties: Record<string, object> = {};
  const required: string[] = [];

  for (const [name, te] of fields) {
    properties[name] = typeExprToSchema(te);
    required.push(name);
  }

  return {
    type: 'object',
    properties,
    required,
    additionalProperties: false,
  };
}
