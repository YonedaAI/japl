import type { ProcessId } from "./types.js";
import type { Mailbox } from "./mailbox.js";

export type ProcessState = "running" | "waiting" | "done" | "failed";

export type CrashReason =
  | { _tag: "Normal" }
  | { _tag: "Error"; message: string; stack?: string }
  | { _tag: "Killed" }
  | { _tag: "LinkedCrash"; pid: ProcessId; reason: CrashReason };

export interface ProcessContext {
  id: ProcessId;
  state: ProcessState;
  mailbox: Mailbox<unknown>;
  parent: ProcessId | null;
  links: Set<ProcessId>;
  monitors: Set<ProcessId>;
  crashReason?: CrashReason;
}
