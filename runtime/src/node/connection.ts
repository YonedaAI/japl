// TCP connection management for JAPL distributed runtime.

import * as net from "node:net";
import type { WireMessage } from "../wire/protocol.js";
import { MsgType } from "../wire/protocol.js";
import { encodeFrame, FrameReader } from "../wire/frame.js";
import type { NodeConfig, NodeId } from "./node.js";
import { parseAddress } from "./node.js";
import {
  createHandshakeMessage,
  createHandshakeAck,
  createHandshakeNack,
  verifyHandshake,
} from "./handshake.js";
import { Reconnector } from "./reconnect.js";

export interface Connection {
  nodeId: NodeId;
  socket: net.Socket;
  state: "connecting" | "handshaking" | "connected" | "disconnected";
  lastPing: number;
  lastPong: number;
}

export interface ConnectionCallbacks {
  onMessage: (from: string, msg: WireMessage) => void;
  onNodeUp: (nodeName: string) => void;
  onNodeDown: (nodeName: string) => void;
}

export class ConnectionManager {
  private connections: Map<string, Connection> = new Map(); // node name -> connection
  private server: net.Server | null = null;
  private config: NodeConfig;
  private onMessage: (from: string, msg: WireMessage) => void;
  private onNodeUp: (nodeName: string) => void;
  private onNodeDown: (nodeName: string) => void;
  private reconnectors: Map<string, Reconnector> = new Map();
  private pingInterval: ReturnType<typeof setInterval> | null = null;

  constructor(config: NodeConfig, callbacks: ConnectionCallbacks) {
    this.config = config;
    this.onMessage = callbacks.onMessage;
    this.onNodeUp = callbacks.onNodeUp;
    this.onNodeDown = callbacks.onNodeDown;
  }

  /** Start listening for incoming connections. */
  async listen(): Promise<void> {
    if (!this.config.listen) return;
    const { host, port } = parseAddress(this.config.listen);

    this.server = net.createServer((socket) => {
      this.handleIncomingConnection(socket);
    });

    return new Promise<void>((resolve, reject) => {
      this.server!.on("error", reject);
      this.server!.listen(port, host, () => {
        this.server!.removeListener("error", reject);
        resolve();
      });
    });
  }

  /** Handle a new incoming TCP connection (wait for HANDSHAKE). */
  private handleIncomingConnection(socket: net.Socket): void {
    const reader = new FrameReader();
    let handshaken = false;

    const timeout = setTimeout(() => {
      if (!handshaken) {
        socket.destroy();
      }
    }, 5000);

    socket.on("data", (data: Buffer) => {
      const messages = reader.feed(new Uint8Array(data));

      for (const msg of messages) {
        if (!handshaken) {
          if (msg.type !== MsgType.HANDSHAKE) {
            socket.destroy();
            clearTimeout(timeout);
            return;
          }

          const result = verifyHandshake(msg.payload, this.config.cookie);
          if (!result.valid) {
            socket.write(createHandshakeNack(this.config.name, result.reason ?? "auth failed"));
            socket.destroy();
            clearTimeout(timeout);
            return;
          }

          handshaken = true;
          clearTimeout(timeout);
          const remoteName = result.nodeName;

          // Send ACK
          socket.write(createHandshakeAck(this.config.name));

          // Register connection
          const conn: Connection = {
            nodeId: { name: remoteName, host: socket.remoteAddress ?? "unknown", port: socket.remotePort ?? 0 },
            socket,
            state: "connected",
            lastPing: Date.now(),
            lastPong: Date.now(),
          };
          this.connections.set(remoteName, conn);
          this.onNodeUp(remoteName);
        } else {
          // Already handshaken — dispatch message
          const remoteName = this.getNodeNameForSocket(socket);
          if (remoteName) {
            this.handleMessage(remoteName, msg);
          }
        }
      }
    });

    socket.on("close", () => {
      clearTimeout(timeout);
      const nodeName = this.getNodeNameForSocket(socket);
      if (nodeName) {
        this.connections.delete(nodeName);
        this.onNodeDown(nodeName);
      }
    });

    socket.on("error", () => {
      clearTimeout(timeout);
      socket.destroy();
    });
  }

  /** Connect to a remote node by address. */
  async connect(address: string): Promise<void> {
    const { host, port } = parseAddress(address);

    return new Promise<void>((resolve, reject) => {
      const socket = net.createConnection({ host, port }, () => {
        // Connection established; send handshake
        socket.write(createHandshakeMessage(this.config.name, this.config.cookie));
      });

      const reader = new FrameReader();
      let handshaken = false;

      const timeout = setTimeout(() => {
        if (!handshaken) {
          socket.destroy();
          reject(new Error(`Handshake timeout connecting to ${address}`));
        }
      }, 5000);

      socket.on("data", (data: Buffer) => {
        const messages = reader.feed(new Uint8Array(data));

        for (const msg of messages) {
          if (!handshaken) {
            if (msg.type === MsgType.HANDSHAKE_ACK) {
              handshaken = true;
              clearTimeout(timeout);

              const remoteName = msg.fromNode;
              const conn: Connection = {
                nodeId: { name: remoteName, host, port },
                socket,
                state: "connected",
                lastPing: Date.now(),
                lastPong: Date.now(),
              };
              this.connections.set(remoteName, conn);
              this.onNodeUp(remoteName);
              resolve();
            } else if (msg.type === MsgType.HANDSHAKE_NACK) {
              clearTimeout(timeout);
              socket.destroy();
              reject(new Error(`Handshake rejected by ${address}`));
            }
          } else {
            const remoteName = this.getNodeNameForSocket(socket);
            if (remoteName) {
              this.handleMessage(remoteName, msg);
            }
          }
        }
      });

      socket.on("error", (err) => {
        clearTimeout(timeout);
        if (!handshaken) {
          reject(err);
        } else {
          const nodeName = this.getNodeNameForSocket(socket);
          if (nodeName) {
            this.handleNodeDisconnect(nodeName, address);
          }
        }
      });

      socket.on("close", () => {
        clearTimeout(timeout);
        const nodeName = this.getNodeNameForSocket(socket);
        if (nodeName) {
          this.handleNodeDisconnect(nodeName, address);
        }
      });
    });
  }

  /** Connect to all configured peers. */
  async connectToPeers(): Promise<void> {
    for (const addr of this.config.connect ?? []) {
      await this.connect(addr).catch(() => {
        // Start reconnector for failed connections
        this.startReconnector(addr);
      });
    }
  }

  /** Send a wire message to a specific node. */
  send(nodeName: string, msg: WireMessage): boolean {
    const conn = this.connections.get(nodeName);
    if (!conn || conn.state !== "connected") return false;
    try {
      const frame = encodeFrame(msg);
      conn.socket.write(frame);
      return true;
    } catch {
      return false;
    }
  }

  /** Get all connected node names. */
  getConnectedNodes(): string[] {
    return [...this.connections.entries()]
      .filter(([_, c]) => c.state === "connected")
      .map(([name]) => name);
  }

  /** Get the connection for a specific node. */
  getConnection(nodeName: string): Connection | undefined {
    return this.connections.get(nodeName);
  }

  /** Disconnect from a specific node. */
  disconnect(nodeName: string): void {
    const conn = this.connections.get(nodeName);
    if (conn) {
      conn.state = "disconnected";
      conn.socket.destroy();
      this.connections.delete(nodeName);
    }
    // Stop any reconnector for this node
    const reconnector = this.reconnectors.get(nodeName);
    if (reconnector) {
      reconnector.stop();
      this.reconnectors.delete(nodeName);
    }
  }

  /** Shut down everything. */
  async shutdown(): Promise<void> {
    // Stop all reconnectors
    for (const [_, reconnector] of this.reconnectors) {
      reconnector.stop();
    }
    this.reconnectors.clear();

    // Stop ping interval
    if (this.pingInterval !== null) {
      clearInterval(this.pingInterval);
      this.pingInterval = null;
    }

    // Close all connections
    for (const [name] of this.connections) {
      this.disconnect(name);
    }

    // Close server
    if (this.server) {
      await new Promise<void>((resolve) => {
        this.server!.close(() => resolve());
      });
      this.server = null;
    }
  }

  /** Start periodic pings to all connected nodes. */
  startPingLoop(intervalMs: number = 10000): void {
    this.pingInterval = setInterval(() => {
      const now = Date.now();
      for (const [name, conn] of this.connections) {
        if (conn.state === "connected") {
          const pingMsg: WireMessage = {
            type: MsgType.PING,
            fromNode: this.config.name,
            toNode: name,
            payload: new Uint8Array(0),
          };
          this.send(name, pingMsg);
          conn.lastPing = now;
        }
      }
    }, intervalMs);
  }

  // --- Private helpers ---

  private handleMessage(from: string, msg: WireMessage): void {
    if (msg.type === MsgType.PING) {
      // Auto-respond with PONG
      const pongMsg: WireMessage = {
        type: MsgType.PONG,
        fromNode: this.config.name,
        toNode: from,
        payload: new Uint8Array(0),
      };
      this.send(from, pongMsg);
      return;
    }

    if (msg.type === MsgType.PONG) {
      const conn = this.connections.get(from);
      if (conn) {
        conn.lastPong = Date.now();
      }
      return;
    }

    this.onMessage(from, msg);
  }

  private handleNodeDisconnect(nodeName: string, address?: string): void {
    const existed = this.connections.has(nodeName);
    this.connections.delete(nodeName);
    if (existed) {
      this.onNodeDown(nodeName);
    }
    // Start reconnector if we have an address
    if (address) {
      this.startReconnector(address);
    }
  }

  private startReconnector(address: string): void {
    // Don't create duplicate reconnectors
    if (this.reconnectors.has(address)) return;

    const reconnector = new Reconnector(async () => {
      await this.connect(address);
      // If connect succeeds, stop the reconnector
      this.reconnectors.delete(address);
    });
    this.reconnectors.set(address, reconnector);
    reconnector.start();
  }

  private getNodeNameForSocket(socket: net.Socket): string | null {
    for (const [name, conn] of this.connections) {
      if (conn.socket === socket) {
        return name;
      }
    }
    return null;
  }
}
