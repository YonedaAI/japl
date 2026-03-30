// Test JAPL process runtime: spawn, send, receive, self_pid
//
// Since spawn always runs _start, the child process differentiates
// itself by checking its PID: pid 0 is parent, pid 1+ is child.

#[link(wasm_import_module = "japl")]
extern "C" {
    fn spawn(func_idx: i64) -> i64;
    fn send(pid: i64, msg: i64);
    fn receive() -> i64;
    fn self_pid() -> i64;
}

fn main() {
    unsafe {
        let my_pid = self_pid();

        if my_pid == 0 {
            // Parent process
            println!("[parent] pid = {}", my_pid);

            // Spawn a child
            let child_pid = spawn(0);
            println!("[parent] spawned child pid = {}", child_pid);

            // Send a message to the child
            send(child_pid, 42);
            println!("[parent] sent 42 to child {}", child_pid);

            // Wait for reply
            let reply = receive();
            println!("[parent] received reply = {}", reply);
        } else {
            // Child process
            println!("[child] pid = {}", my_pid);

            // Wait for a message
            let msg = receive();
            println!("[child] received msg = {}", msg);

            // Reply to parent (pid 0) with msg * 2
            let reply = msg * 2;
            send(0, reply);
            println!("[child] sent reply {} to parent", reply);
        }
    }
}
