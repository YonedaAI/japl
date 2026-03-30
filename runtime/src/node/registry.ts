export interface NodeInfo {
  name: string;
  host: string;
  port: number;
  connectedAt: number;
  status: "up" | "suspect" | "down";
}

export class NodeRegistry {
  /** Known nodes. */
  private nodes: Map<string, NodeInfo> = new Map();
  /** Named processes (name -> { node, pid }). */
  private names: Map<string, { node: string; pid: string }> = new Map();

  /** Register a node. */
  addNode(name: string, info: NodeInfo): void {
    this.nodes.set(name, info);
  }

  /** Remove a node and all names registered on it. */
  removeNode(name: string): void {
    this.nodes.delete(name);
    for (const [regName, loc] of this.names) {
      if (loc.node === name) this.names.delete(regName);
    }
  }

  /** Get a node by name. */
  getNode(name: string): NodeInfo | undefined {
    return this.nodes.get(name);
  }

  /** Get all known nodes. */
  getAllNodes(): Map<string, NodeInfo> {
    return new Map(this.nodes);
  }

  /** Register a named process. */
  register(name: string, node: string, pid: string): void {
    this.names.set(name, { node, pid });
  }

  /** Lookup a named process. */
  lookup(name: string): { node: string; pid: string } | undefined {
    return this.names.get(name);
  }

  /** Unregister a named process. */
  unregister(name: string): void {
    this.names.delete(name);
  }

  /** Get all registered names. */
  getRegisteredNames(): string[] {
    return [...this.names.keys()];
  }
}
