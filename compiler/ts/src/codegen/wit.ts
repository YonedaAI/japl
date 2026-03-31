// ─── WIT (WebAssembly Interface Types) Generator ───
// Generates WIT interface definitions from JAPL type declarations.
// WIT defines typed contracts between WASM components.

import type { Module, Decl, TypeExpr, Variant, Field, Param } from '../parser/ast.js';

/** Convert PascalCase or camelCase to kebab-case. */
function toKebab(name: string): string {
  return name
    .replace(/([a-z0-9])([A-Z])/g, '$1-$2')
    .replace(/([A-Z])([A-Z][a-z])/g, '$1-$2')
    .toLowerCase();
}

/** Map a JAPL type expression to a WIT type string. */
function typeExprToWit(te: TypeExpr): string {
  switch (te.kind) {
    case 'tnamed':
      return namedTypeToWit(te.name, te.args);
    case 'ttuple':
      if (te.elements.length === 0) return 'tuple<>';
      return `tuple<${te.elements.map(typeExprToWit).join(', ')}>`;
    case 'tunit':
      return 'tuple<>';
    case 'tfn':
      // Functions can't be directly expressed as WIT types
      return 'string';
    case 'tvar':
      // Unconstrained type variable — fall back to string
      return 'string';
    case 'trecord':
      // Inline records can't be directly expressed in WIT; fall back
      return 'string';
    default:
      return 'string';
  }
}

function namedTypeToWit(name: string, args: TypeExpr[]): string {
  switch (name) {
    case 'Int':
      return 's64';
    case 'Float':
      return 'f64';
    case 'String':
      return 'string';
    case 'Bool':
      return 'bool';
    case 'Byte':
      return 'u8';
    case 'Unit':
      return 'tuple<>';
    case 'List': {
      const inner = args.length > 0 ? typeExprToWit(args[0]) : 'string';
      return `list<${inner}>`;
    }
    case 'Option': {
      const inner = args.length > 0 ? typeExprToWit(args[0]) : 'string';
      return `option<${inner}>`;
    }
    default:
      // User-defined type — convert to kebab-case reference
      return toKebab(name);
  }
}

/** Convert a variant (sum type constructor) to WIT variant case. */
function variantCaseToWit(v: Variant): string {
  const name = toKebab(v.name);
  if (v.fields.length === 0) {
    return `    ${name},`;
  }
  if (v.fields.length === 1) {
    return `    ${name}(${typeExprToWit(v.fields[0])}),`;
  }
  // Multiple fields → tuple
  const inner = v.fields.map(typeExprToWit).join(', ');
  return `    ${name}(tuple<${inner}>),`;
}

/** Convert a sum type declaration to WIT variant definition. */
function sumTypeToWit(name: string, variants: Variant[]): string {
  const witName = toKebab(name);
  const lines: string[] = [];
  lines.push(`    variant ${witName} {`);
  for (const v of variants) {
    lines.push(variantCaseToWit(v));
  }
  lines.push('    }');
  return lines.join('\n');
}

/** Convert a record type declaration to WIT record definition. */
function recordTypeToWit(name: string, fields: Field[]): string {
  const witName = toKebab(name);
  const lines: string[] = [];
  lines.push(`    record ${witName} {`);
  for (const f of fields) {
    lines.push(`        ${toKebab(f.name)}: ${typeExprToWit(f.type)},`);
  }
  lines.push('    }');
  return lines.join('\n');
}

/** Convert a function declaration to WIT function signature. */
function fnToWit(decl: Extract<Decl, { kind: 'fn' }>): string {
  const name = toKebab(decl.name);
  const params = decl.params.map(p => {
    const pType = p.type ? typeExprToWit(p.type) : 'string';
    return `${toKebab(p.name)}: ${pType}`;
  }).join(', ');

  const ret = decl.returnType ? typeExprToWit(decl.returnType) : undefined;

  if (ret && ret !== 'tuple<>') {
    return `    ${name}: func(${params}) -> ${ret};`;
  }
  return `    ${name}: func(${params});`;
}

/** Generate a complete WIT file from a JAPL module AST. */
export function generateWit(module: Module, packageName: string): string {
  const lines: string[] = [];
  lines.push(`package japl:${packageName};`);
  lines.push('');

  // Collect type and pub fn declarations
  const types = module.decls.filter(
    (d): d is Extract<Decl, { kind: 'type' }> | Extract<Decl, { kind: 'record_type' }> =>
      d.kind === 'type' || d.kind === 'record_type'
  );
  const fns = module.decls.filter(
    (d): d is Extract<Decl, { kind: 'fn' }> =>
      d.kind === 'fn' && d.pub
  );

  if (types.length > 0 || fns.length > 0) {
    lines.push(`interface ${packageName} {`);

    for (const decl of types) {
      if (decl.kind === 'type') {
        lines.push(sumTypeToWit(decl.name, decl.variants));
      } else if (decl.kind === 'record_type') {
        lines.push(recordTypeToWit(decl.name, decl.fields));
      }
    }

    for (const decl of fns) {
      lines.push(fnToWit(decl));
    }

    lines.push('}');
  }

  lines.push('');
  lines.push(`world ${packageName}-world {`);
  if (types.length > 0 || fns.length > 0) {
    lines.push(`    export ${packageName};`);
  }
  lines.push('}');

  return lines.join('\n') + '\n';
}
