// ─── Simple TOML-subset parser for japl.toml ───
// Supports [sections] and key = "value" pairs with string values.

export interface NodeConfig {
  name?: string;
  listen?: string;
  cookie?: string;
  connect?: string[];
}

export interface JaplConfig {
  package?: {
    name?: string;
    version?: string;
    entry?: string;
  };
  dependencies?: Record<string, string>;
  'dev-dependencies'?: Record<string, string>;
  node?: NodeConfig;
  [section: string]: Record<string, string> | NodeConfig | undefined;
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

    // Key-value pair: key = "value" or key = value or key = ["a", "b"]
    const kvMatch = line.match(/^([a-zA-Z0-9_-]+)\s*=\s*(.+)$/);
    if (kvMatch) {
      const key = kvMatch[1];
      const rawValue = kvMatch[2].trim();

      if (currentSection) {
        if (!config[currentSection]) {
          config[currentSection] = {};
        }

        // Handle array values: ["a", "b"]
        if (rawValue.startsWith('[') && rawValue.endsWith(']')) {
          const inner = rawValue.slice(1, -1).trim();
          const items = inner === '' ? [] : inner.split(',').map(item => {
            const trimmed = item.trim();
            if ((trimmed.startsWith('"') && trimmed.endsWith('"')) ||
                (trimmed.startsWith("'") && trimmed.endsWith("'"))) {
              return trimmed.slice(1, -1);
            }
            return trimmed;
          });
          if (currentSection === 'node') {
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            (config[currentSection] as any)[key] = items;
          } else {
            // For non-node sections, store as comma-separated string
            (config[currentSection] as Record<string, string>)[key] = items.join(',');
          }
        } else {
          let value = rawValue;
          // Strip surrounding quotes
          if ((value.startsWith('"') && value.endsWith('"')) ||
              (value.startsWith("'") && value.endsWith("'"))) {
            value = value.slice(1, -1);
          }

          if (currentSection === 'node') {
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            (config[currentSection] as any)[key] = value;
          } else {
            (config[currentSection] as Record<string, string>)[key] = value;
          }
        }
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
