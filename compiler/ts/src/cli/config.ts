// ─── Simple TOML-subset parser for japl.toml ───
// Supports [sections] and key = "value" pairs with string values.

export interface JaplConfig {
  package?: {
    name?: string;
    version?: string;
    entry?: string;
  };
  dependencies?: Record<string, string>;
  'dev-dependencies'?: Record<string, string>;
  [section: string]: Record<string, string> | undefined;
}

export function parseConfig(source: string): JaplConfig {
  const config: JaplConfig = {};
  let currentSection: string | null = null;

  const lines = source.split('\n');

  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i];
    const line = raw.trim();

    // Skip empty lines and comments
    if (line === '' || line.startsWith('#')) {
      continue;
    }

    // Section header: [name]
    const sectionMatch = line.match(/^\[([a-zA-Z0-9_-]+)\]$/);
    if (sectionMatch) {
      currentSection = sectionMatch[1];
      if (!config[currentSection]) {
        config[currentSection] = {};
      }
      continue;
    }

    // Key-value pair: key = "value" or key = value
    const kvMatch = line.match(/^([a-zA-Z0-9_-]+)\s*=\s*(.+)$/);
    if (kvMatch) {
      const key = kvMatch[1];
      let value = kvMatch[2].trim();

      // Strip surrounding quotes
      if ((value.startsWith('"') && value.endsWith('"')) ||
          (value.startsWith("'") && value.endsWith("'"))) {
        value = value.slice(1, -1);
      }

      if (currentSection) {
        if (!config[currentSection]) {
          config[currentSection] = {};
        }
        (config[currentSection] as Record<string, string>)[key] = value;
      }
      continue;
    }

    // Unrecognized line — skip silently
  }

  return config;
}

export function loadConfig(filePath: string): JaplConfig {
  const fs = require('node:fs');
  const source = fs.readFileSync(filePath, 'utf-8');
  return parseConfig(source);
}
