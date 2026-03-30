// JAPL Process Runtime (inlined)
const __japl_processes = new Map();
let __japl_pid_counter = 0;
let __japl_current_pid = 'main';
function spawn(fn) {
  const pid = 'pid-' + (++__japl_pid_counter);
  const mailbox = [];
  const waiters = [];
  __japl_processes.set(pid, { mailbox, waiters });
  Promise.resolve().then(async () => {
    __japl_current_pid = pid;
    await fn();
  }).catch(e => console.error('[process ' + pid + ' crashed]', e));
  return pid;
}
function send(pid, msg) {
  const proc = __japl_processes.get(pid);
  if (!proc) { console.error('send: unknown pid', pid); return; }
  if (proc.waiters.length > 0) { proc.waiters.shift()(msg); }
  else { proc.mailbox.push(msg); }
}
function receive() {
  const proc = __japl_processes.get(__japl_current_pid);
  if (!proc) return Promise.reject(new Error('receive: no process context'));
  if (proc.mailbox.length > 0) return Promise.resolve(proc.mailbox.shift());
  return new Promise(resolve => proc.waiters.push(resolve));
}

const println = (...args: any[]) => console.log(...args);
const print = (...args: any[]) => process.stdout.write(args.join(''));
const show = (v: any): string => typeof v === 'string' ? v : JSON.stringify(v);
const int_to_string = (n: number): string => String(n);
const string_length = (s: string): number => s.length;

type Msg = { _tag: "Inc" } | { _tag: "Get" };
const Inc: Msg = { _tag: "Inc" };
const Get: Msg = { _tag: "Get" };

function counter(n) {
  println("Counter value: " + show(n));
  return (async () => {
    const __msg = await receive();
    switch (__msg._tag) {
      case "Inc": {
        return counter(n + 1);
        break;
      }
      case "Get": {
        return println("Final: " + show(n));
        break;
      }
    }
  })();
}

function main() {
  println("Starting concurrent counter...");
  const pid = spawn(async () => counter(0));
  send(pid, { _tag: "Inc" });
  send(pid, { _tag: "Inc" });
  send(pid, { _tag: "Inc" });
  send(pid, { _tag: "Get" });
  return println("Messages sent.");
}

main();
