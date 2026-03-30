// Node identity and configuration types for JAPL distributed runtime.

export interface NodeConfig {
  name: string;       // e.g., "alpha"
  listen?: string;    // e.g., ":9000" or "0.0.0.0:9000"
  connect?: string[]; // e.g., ["beta:9001"]
  cookie: string;     // shared secret for auth
}

export interface NodeId {
  name: string;
  host: string;
  port: number;
}

export function parseAddress(addr: string): { host: string; port: number } {
  const colonIdx = addr.lastIndexOf(":");
  if (colonIdx === -1) {
    throw new Error(`Invalid address: "${addr}" — expected "host:port" or ":port"`);
  }
  const hostPart = addr.slice(0, colonIdx);
  const portStr = addr.slice(colonIdx + 1);
  const port = parseInt(portStr, 10);
  if (isNaN(port) || port < 0 || port > 65535) {
    throw new Error(`Invalid port in address: "${addr}"`);
  }
  const host = hostPart === "" ? "0.0.0.0" : hostPart;
  return { host, port };
}
