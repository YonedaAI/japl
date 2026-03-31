use std::collections::HashMap;
use crate::ir::*;

pub struct WatEmitter {
    output: String,
    indent: usize,
    module: IrModule,
    // Track which functions need to be in the table (for closures/indirect calls)
    table_entries: Vec<String>,
    // Scratch slot counter for nested allocations (each slot is 4 bytes at 20+4*slot)
    scratch_slot: usize,
    // Foreign function signatures for call-site type conversion
    foreign_sigs: HashMap<String, IrForeignImport>,
    // Track which user functions return a value
    fn_has_return: HashMap<String, bool>,
}

impl WatEmitter {
    pub fn new(module: IrModule) -> Self {
        // Collect closure body functions for table
        let mut table_entries = Vec::new();
        for f in &module.functions {
            if f.is_closure_body {
                table_entries.push(f.name.clone());
            }
        }

        let mut foreign_sigs = HashMap::new();
        for fi in &module.foreign_imports {
            foreign_sigs.insert(fi.name.clone(), fi.clone());
        }

        let mut fn_has_return = HashMap::new();
        for f in &module.functions {
            fn_has_return.insert(f.name.clone(), f.has_return);
        }

        WatEmitter {
            output: String::new(),
            indent: 0,
            module,
            table_entries,
            scratch_slot: 0,
            foreign_sigs,
            fn_has_return,
        }
    }

    fn line(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn push(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
        self.output.push_str(s);
        self.output.push('\n');
        self.indent += 1;
    }

    fn pop(&mut self, s: &str) {
        self.indent -= 1;
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
        self.output.push_str(s);
        self.output.push('\n');
    }

    pub fn emit(mut self) -> String {
        self.push("(module");

        // WASI fd_write import (imports must come first)
        self.line("(import \"wasi_snapshot_preview1\" \"fd_write\" (func $fd_write (param i32 i32 i32 i32) (result i32)))");

        // Foreign imports
        for fi in self.module.foreign_imports.clone() {
            let params: String = fi.param_types.iter().map(|t| {
                match t {
                    WasmType::I32 => " (param i32)",
                    WasmType::I64 => " (param i64)",
                }
            }).collect();
            let result: String = fi.return_types.iter().map(|t| {
                match t {
                    WasmType::I32 => " (result i32)",
                    WasmType::I64 => " (result i64)",
                }
            }).collect();
            self.line(&format!(
                "(import \"{}\" \"{}\" (func ${}{}{}))",
                fi.module, fi.name, fi.name, params, result
            ));
        }

        // Memory - start with 10 pages (640KB), growable to 65536 pages (4GB)
        self.line("(memory (export \"memory\") 10 65536)");

        // Globals
        if self.module.uses_processes {
            self.line(&format!("(global $heap_ptr (export \"heap_ptr\") (mut i32) (i32.const {}))", self.module.heap_start));
        } else {
            self.line(&format!("(global $heap_ptr (mut i32) (i32.const {}))", self.module.heap_start));
        }

        // Data segments for strings
        let data_lines: Vec<String> = self.module.string_data.iter().map(|sd| {
            let mut escaped = String::new();
            let len_bytes = (sd.length as u32).to_le_bytes();
            for b in &len_bytes {
                escaped.push_str(&format!("\\{:02x}", b));
            }
            for b in sd.content.as_bytes() {
                if *b >= 0x20 && *b < 0x7f && *b != b'"' && *b != b'\\' {
                    escaped.push(*b as char);
                } else {
                    escaped.push_str(&format!("\\{:02x}", b));
                }
            }
            format!("(data (i32.const {}) \"{}\")", sd.offset, escaped)
        }).collect();
        for dl in &data_lines {
            self.line(dl);
        }

        // Function table (for closures and indirect calls)
        if !self.table_entries.is_empty() {
            let entries: String = self.table_entries.iter()
                .map(|e| format!(" ${}", e))
                .collect();
            self.line(&format!(
                "(table {} funcref)",
                self.table_entries.len()
            ));
            self.line(&format!(
                "(elem (i32.const 0){})",
                entries
            ));
            // Type for closure calls: closure_ptr + varying args -> i64
            // We need types for different arities
            self.line("(type $closure_0 (func (param i64) (result i64)))");
            self.line("(type $closure_1 (func (param i64 i64) (result i64)))");
            self.line("(type $closure_2 (func (param i64 i64 i64) (result i64)))");
            self.line("(type $closure_3 (func (param i64 i64 i64 i64) (result i64)))");
        }

        // Builtin: $alloc (bump allocator)
        self.emit_alloc();

        // Builtin: $println
        self.emit_println();

        // Builtin: $show_int
        self.emit_show_int();

        // Builtin: $show_bool
        self.emit_show_bool();

        // Builtin: $string_concat
        self.emit_string_concat();

        // User functions
        let functions = self.module.functions.clone();
        for func in &functions {
            self.emit_function(func);
        }

        // _start function that calls main
        self.push("(func (export \"_start\")");
        self.line("call $main");
        // If main returns a value, drop it
        let main_fn = functions.iter().find(|f| f.name == "main");
        if let Some(mf) = main_fn {
            if mf.has_return {
                self.line("drop");
            }
        }
        self.pop(")");

        // __process_entry for spawned processes (runtime calls this)
        if self.module.uses_processes {
            self.push("(func (export \"__process_entry\") (param $closure_ptr i64)");
            // The closure struct: [i64 table_index][i64 capture_0]...
            // Push closure_ptr as the first (and only) arg for $closure_0 type
            self.line("local.get $closure_ptr");
            // Read table index from closure struct
            self.line("local.get $closure_ptr");
            self.line("i32.wrap_i64");
            self.line("i64.load");
            self.line("i32.wrap_i64");
            self.line("call_indirect (type $closure_0)");
            self.line("drop");
            self.pop(")");
        }

        // __handle_http export for wasmCloud HTTP handler
        // Emitted when the user defines: fn handle_request(method, path, body) -> String
        let has_http_handler = functions.iter().any(|f| {
            f.name == "handle_request" && f.params.len() == 3 && f.has_return
        });
        if has_http_handler {
            self.emit_http_handler();
        }

        // Constants as globals
        let constants = self.module.constants.clone();
        for (name, val) in &constants {
            self.line(&format!("(global ${} i64 (i64.const {}))", name, val));
        }

        self.pop(")");
        self.output
    }

    fn emit_alloc(&mut self) {
        self.push("(func $alloc (param $size i32) (result i32)");
        self.line("(local $ptr i32)");
        self.line("(local $new_heap i32)");
        self.line("global.get $heap_ptr");
        self.line("local.set $ptr");
        // Align size to 8 bytes
        self.line("local.get $size");
        self.line("i32.const 7");
        self.line("i32.add");
        self.line("i32.const -8");
        self.line("i32.and");
        self.line("global.get $heap_ptr");
        self.line("i32.add");
        self.line("local.set $new_heap");
        // Grow memory if needed: if new_heap > memory.size * 64KB
        self.push("block $no_grow");
        self.line("local.get $new_heap");
        self.line("memory.size");
        self.line("i32.const 16"); // 65536
        self.line("i32.shl");
        self.line("i32.lt_u");
        self.line("br_if $no_grow");
        // Calculate pages needed: (new_heap - current_size + 65535) / 65536
        self.line("local.get $new_heap");
        self.line("memory.size");
        self.line("i32.const 16");
        self.line("i32.shl");
        self.line("i32.sub");
        self.line("i32.const 65535");
        self.line("i32.add");
        self.line("i32.const 16");
        self.line("i32.shr_u");
        self.line("i32.const 1");
        self.line("i32.add");
        self.line("memory.grow");
        self.line("drop");
        self.pop("end");
        self.line("local.get $new_heap");
        self.line("global.set $heap_ptr");
        self.line("local.get $ptr");
        self.pop(")");
    }

    fn emit_println(&mut self) {
        // println takes an i64 which is a string pointer (i32 really)
        self.push("(func $println (param $s i64)");
        self.line("(local $ptr i32)");
        self.line("(local $len i32)");
        // Get string pointer
        self.line("local.get $s");
        self.line("i32.wrap_i64");
        self.line("local.set $ptr");
        // Read length from [ptr]
        self.line("local.get $ptr");
        self.line("i32.load");
        self.line("local.set $len");
        // Write iov at address 0: [ptr+4, len]
        self.line("i32.const 0");
        self.line("local.get $ptr");
        self.line("i32.const 4");
        self.line("i32.add");
        self.line("i32.store");
        self.line("i32.const 4");
        self.line("local.get $len");
        self.line("i32.store");
        // fd_write(1, iov=0, iovs_count=1, nwritten=8)
        self.line("i32.const 1");
        self.line("i32.const 0");
        self.line("i32.const 1");
        self.line("i32.const 8");
        self.line("call $fd_write");
        self.line("drop");
        // Write newline
        self.line("i32.const 12");
        self.line("i32.const 10"); // '\n'
        self.line("i32.store8");
        self.line("i32.const 0");
        self.line("i32.const 12");
        self.line("i32.store");
        self.line("i32.const 4");
        self.line("i32.const 1");
        self.line("i32.store");
        self.line("i32.const 1");
        self.line("i32.const 0");
        self.line("i32.const 1");
        self.line("i32.const 16");
        self.line("call $fd_write");
        self.line("drop");
        self.pop(")");
    }

    fn emit_show_int(&mut self) {
        // Convert i64 to decimal string, return string pointer
        self.push("(func $show_int (param $n i64) (result i64)");
        self.line("(local $ptr i32)");
        self.line("(local $buf i32)");
        self.line("(local $len i32)");
        self.line("(local $neg i32)");
        self.line("(local $digit i64)");
        self.line("(local $tmp i64)");
        self.line("(local $start i32)");
        self.line("(local $end i32)");
        self.line("(local $swap i32)");

        // Allocate 24 bytes for temp buffer (max i64 is 20 digits + sign + len header)
        self.line("i32.const 32");
        self.line("call $alloc");
        self.line("local.set $ptr");

        // buf starts at ptr+4 (after length field)
        self.line("local.get $ptr");
        self.line("i32.const 4");
        self.line("i32.add");
        self.line("local.set $buf");

        // Handle negative
        self.line("i32.const 0");
        self.line("local.set $neg");
        self.line("local.get $n");
        self.line("i64.const 0");
        self.line("i64.lt_s");
        self.push("if");
        self.line("i32.const 1");
        self.line("local.set $neg");
        self.line("i64.const 0");
        self.line("local.get $n");
        self.line("i64.sub");
        self.line("local.set $n");
        self.pop("end");

        // Handle 0
        self.line("local.get $n");
        self.line("i64.const 0");
        self.line("i64.eq");
        self.push("if");
        self.line("local.get $buf");
        self.line("i32.const 48"); // '0'
        self.line("i32.store8");
        self.line("local.get $ptr");
        self.line("i32.const 1");
        self.line("i32.store");
        self.line("local.get $ptr");
        self.line("i64.extend_i32_u");
        self.line("return");
        self.pop("end");

        // Convert digits (in reverse)
        self.line("i32.const 0");
        self.line("local.set $len");
        self.push("block $done");
        self.push("loop $loop");
        self.line("local.get $n");
        self.line("i64.const 0");
        self.line("i64.eq");
        self.line("br_if $done");

        self.line("local.get $n");
        self.line("i64.const 10");
        self.line("i64.rem_u");
        self.line("local.set $digit");

        self.line("local.get $buf");
        self.line("local.get $len");
        self.line("i32.add");
        self.line("local.get $digit");
        self.line("i32.wrap_i64");
        self.line("i32.const 48");
        self.line("i32.add");
        self.line("i32.store8");

        self.line("local.get $len");
        self.line("i32.const 1");
        self.line("i32.add");
        self.line("local.set $len");

        self.line("local.get $n");
        self.line("i64.const 10");
        self.line("i64.div_u");
        self.line("local.set $n");

        self.line("br $loop");
        self.pop("end");
        self.pop("end");

        // If negative, add '-' at the end (we'll reverse)
        self.line("local.get $neg");
        self.push("if");
        self.line("local.get $buf");
        self.line("local.get $len");
        self.line("i32.add");
        self.line("i32.const 45"); // '-'
        self.line("i32.store8");
        self.line("local.get $len");
        self.line("i32.const 1");
        self.line("i32.add");
        self.line("local.set $len");
        self.pop("end");

        // Reverse the string in-place
        self.line("i32.const 0");
        self.line("local.set $start");
        self.line("local.get $len");
        self.line("i32.const 1");
        self.line("i32.sub");
        self.line("local.set $end");

        self.push("block $rdone");
        self.push("loop $rloop");
        self.line("local.get $start");
        self.line("local.get $end");
        self.line("i32.ge_s");
        self.line("br_if $rdone");

        // swap buf[start] and buf[end]
        self.line("local.get $buf");
        self.line("local.get $start");
        self.line("i32.add");
        self.line("i32.load8_u");
        self.line("local.set $swap");

        self.line("local.get $buf");
        self.line("local.get $start");
        self.line("i32.add");
        self.line("local.get $buf");
        self.line("local.get $end");
        self.line("i32.add");
        self.line("i32.load8_u");
        self.line("i32.store8");

        self.line("local.get $buf");
        self.line("local.get $end");
        self.line("i32.add");
        self.line("local.get $swap");
        self.line("i32.store8");

        self.line("local.get $start");
        self.line("i32.const 1");
        self.line("i32.add");
        self.line("local.set $start");
        self.line("local.get $end");
        self.line("i32.const 1");
        self.line("i32.sub");
        self.line("local.set $end");
        self.line("br $rloop");
        self.pop("end");
        self.pop("end");

        // Store length
        self.line("local.get $ptr");
        self.line("local.get $len");
        self.line("i32.store");

        self.line("local.get $ptr");
        self.line("i64.extend_i32_u");
        self.pop(")");
    }

    fn emit_show_bool(&mut self) {
        self.push("(func $show_bool (param $b i64) (result i64)");
        self.line("(local $ptr i32)");
        self.line("local.get $b");
        self.line("i64.const 0");
        self.line("i64.ne");
        self.push("if (result i64)");
        // "true" - allocate and write
        self.line("i32.const 8");
        self.line("call $alloc");
        self.line("local.set $ptr");
        self.line("local.get $ptr");
        self.line("i32.const 4");
        self.line("i32.store"); // length = 4
        self.line("local.get $ptr");
        self.line("i32.const 4");
        self.line("i32.add");
        self.line("i32.const 1702195828"); // "true" as i32 LE
        self.line("i32.store");
        self.line("local.get $ptr");
        self.line("i64.extend_i32_u");
        self.push("else");
        // "false" - allocate and write
        self.line("i32.const 12");
        self.line("call $alloc");
        self.line("local.set $ptr");
        self.line("local.get $ptr");
        self.line("i32.const 5");
        self.line("i32.store"); // length = 5
        self.line("local.get $ptr");
        self.line("i32.const 4");
        self.line("i32.add");
        self.line("i32.const 1936482662"); // "fals" as i32 LE: f=102,a=97,l=108,s=115
        self.line("i32.store");
        self.line("local.get $ptr");
        self.line("i32.const 8");
        self.line("i32.add");
        self.line("i32.const 101"); // "e"
        self.line("i32.store8");
        self.line("local.get $ptr");
        self.line("i64.extend_i32_u");
        self.pop("end");
        self.pop(")");
    }

    fn emit_string_concat(&mut self) {
        self.push("(func $string_concat (param $a i64) (param $b i64) (result i64)");
        self.line("(local $pa i32)");
        self.line("(local $pb i32)");
        self.line("(local $la i32)");
        self.line("(local $lb i32)");
        self.line("(local $ptr i32)");
        self.line("(local $i i32)");

        self.line("local.get $a");
        self.line("i32.wrap_i64");
        self.line("local.set $pa");
        self.line("local.get $b");
        self.line("i32.wrap_i64");
        self.line("local.set $pb");

        // Load lengths
        self.line("local.get $pa");
        self.line("i32.load");
        self.line("local.set $la");
        self.line("local.get $pb");
        self.line("i32.load");
        self.line("local.set $lb");

        // Allocate: 4 + la + lb
        self.line("i32.const 4");
        self.line("local.get $la");
        self.line("i32.add");
        self.line("local.get $lb");
        self.line("i32.add");
        self.line("call $alloc");
        self.line("local.set $ptr");

        // Store total length
        self.line("local.get $ptr");
        self.line("local.get $la");
        self.line("local.get $lb");
        self.line("i32.add");
        self.line("i32.store");

        // Copy first string
        self.line("i32.const 0");
        self.line("local.set $i");
        self.push("block $d1");
        self.push("loop $l1");
        self.line("local.get $i");
        self.line("local.get $la");
        self.line("i32.ge_u");
        self.line("br_if $d1");
        self.line("local.get $ptr");
        self.line("i32.const 4");
        self.line("i32.add");
        self.line("local.get $i");
        self.line("i32.add");
        self.line("local.get $pa");
        self.line("i32.const 4");
        self.line("i32.add");
        self.line("local.get $i");
        self.line("i32.add");
        self.line("i32.load8_u");
        self.line("i32.store8");
        self.line("local.get $i");
        self.line("i32.const 1");
        self.line("i32.add");
        self.line("local.set $i");
        self.line("br $l1");
        self.pop("end");
        self.pop("end");

        // Copy second string
        self.line("i32.const 0");
        self.line("local.set $i");
        self.push("block $d2");
        self.push("loop $l2");
        self.line("local.get $i");
        self.line("local.get $lb");
        self.line("i32.ge_u");
        self.line("br_if $d2");
        self.line("local.get $ptr");
        self.line("i32.const 4");
        self.line("i32.add");
        self.line("local.get $la");
        self.line("i32.add");
        self.line("local.get $i");
        self.line("i32.add");
        self.line("local.get $pb");
        self.line("i32.const 4");
        self.line("i32.add");
        self.line("local.get $i");
        self.line("i32.add");
        self.line("i32.load8_u");
        self.line("i32.store8");
        self.line("local.get $i");
        self.line("i32.const 1");
        self.line("i32.add");
        self.line("local.set $i");
        self.line("br $l2");
        self.pop("end");
        self.pop("end");

        self.line("local.get $ptr");
        self.line("i64.extend_i32_u");
        self.pop(")");
    }

    fn emit_http_handler(&mut self) {
        // __handle_http: bridge from raw ptr/len pairs to JAPL's string-as-i64 calling convention
        // Signature: (method_ptr, method_len, path_ptr, path_len, body_ptr, body_len) -> (resp_ptr, resp_len)
        self.push("(func $__handle_http (export \"__handle_http\") (param $method_ptr i32) (param $method_len i32) (param $path_ptr i32) (param $path_len i32) (param $body_ptr i32) (param $body_len i32) (result i32 i32)");
        self.line("(local $m i64)");
        self.line("(local $p i64)");
        self.line("(local $b i64)");
        self.line("(local $result i64)");
        self.line("(local $str_ptr i32)");
        self.line("(local $r_ptr i32)");
        self.line("(local $r_len i32)");
        self.line("(local $i i32)");

        // Build JAPL string for method: allocate [len][bytes], copy data
        self.emit_http_pack_string("$method_ptr", "$method_len", "$m");
        // Build JAPL string for path
        self.emit_http_pack_string("$path_ptr", "$path_len", "$p");
        // Build JAPL string for body
        self.emit_http_pack_string("$body_ptr", "$body_len", "$b");

        // Call handle_request(method, path, body)
        self.line("local.get $m");
        self.line("local.get $p");
        self.line("local.get $b");
        self.line("call $handle_request");
        self.line("local.set $result");

        // Unpack result: i64 -> ptr (low 32 bits), string is [i32 len][bytes] at ptr
        self.line("local.get $result");
        self.line("i32.wrap_i64");
        self.line("local.set $r_ptr");
        // Read length from [r_ptr]
        self.line("local.get $r_ptr");
        self.line("i32.load");
        self.line("local.set $r_len");
        // Return (data_ptr = r_ptr + 4, length = r_len)
        self.line("local.get $r_ptr");
        self.line("i32.const 4");
        self.line("i32.add");
        self.line("local.get $r_len");
        self.pop(")");
    }

    fn emit_http_pack_string(&mut self, ptr_local: &str, len_local: &str, dest_local: &str) {
        // Allocate 4 + len bytes, store len at offset 0, copy bytes at offset 4
        self.line(&format!("i32.const 4"));
        self.line(&format!("local.get {}", len_local));
        self.line("i32.add");
        self.line("call $alloc");
        self.line("local.set $str_ptr");

        // Store length
        self.line("local.get $str_ptr");
        self.line(&format!("local.get {}", len_local));
        self.line("i32.store");

        // Copy bytes: loop from 0 to len
        self.line("i32.const 0");
        self.line("local.set $i");
        self.push(&format!("block $pack_done_{}", dest_local.trim_start_matches('$')));
        self.push(&format!("loop $pack_loop_{}", dest_local.trim_start_matches('$')));
        self.line("local.get $i");
        self.line(&format!("local.get {}", len_local));
        self.line("i32.ge_u");
        self.line(&format!("br_if $pack_done_{}", dest_local.trim_start_matches('$')));

        // dest[4+i] = src[ptr+i]
        self.line("local.get $str_ptr");
        self.line("i32.const 4");
        self.line("i32.add");
        self.line("local.get $i");
        self.line("i32.add");
        self.line(&format!("local.get {}", ptr_local));
        self.line("local.get $i");
        self.line("i32.add");
        self.line("i32.load8_u");
        self.line("i32.store8");

        self.line("local.get $i");
        self.line("i32.const 1");
        self.line("i32.add");
        self.line("local.set $i");
        self.line(&format!("br $pack_loop_{}", dest_local.trim_start_matches('$')));
        self.pop("end");
        self.pop("end");

        // Set dest = str_ptr as i64
        self.line("local.get $str_ptr");
        self.line("i64.extend_i32_u");
        self.line(&format!("local.set {}", dest_local));
    }

    fn emit_function(&mut self, func: &IrFunction) {
        let params: String = func.params.iter()
            .map(|p| format!(" (param ${} i64)", p))
            .collect();
        let result = if func.has_return { " (result i64)" } else { "" };

        let export = if func.name == "main" {
            ""
        } else {
            ""
        };

        self.push(&format!("(func ${}{}{}{}", func.name, export, params, result));

        // Declare locals
        for local in &func.locals {
            self.line(&format!("(local ${} i64)", local));
        }

        self.emit_expr(&func.body, func.has_return);

        self.pop(")");
    }

    fn emit_expr(&mut self, expr: &IrExpr, need_value: bool) {
        match expr {
            IrExpr::I64Const(n) => {
                self.line(&format!("i64.const {}", n));
                if !need_value {
                    self.line("drop");
                }
            }
            IrExpr::StringConst(offset, _len) => {
                // Return pointer to the string (which starts with 4-byte length)
                self.line(&format!("i64.const {}", offset));
                if !need_value {
                    self.line("drop");
                }
            }
            IrExpr::BoolConst(b) => {
                self.line(&format!("i64.const {}", if *b { 1 } else { 0 }));
                if !need_value {
                    self.line("drop");
                }
            }
            IrExpr::LocalGet(name) => {
                self.line(&format!("local.get ${}", name));
                if !need_value {
                    self.line("drop");
                }
            }
            IrExpr::LocalSet(name, val) => {
                self.emit_expr(val, true);
                self.line(&format!("local.set ${}", name));
            }
            IrExpr::Call(name, args) => {
                if name == "$println" {
                    for arg in args {
                        self.emit_expr(arg, true);
                    }
                    self.line("call $println");
                    if need_value {
                        self.line("i64.const 0");
                    }
                } else if let Some(fi) = self.foreign_sigs.get(name).cloned() {
                    // Foreign function call: convert i64 args to actual param types
                    for (i, arg) in args.iter().enumerate() {
                        self.emit_expr(arg, true);
                        if i < fi.param_types.len() && fi.param_types[i] == WasmType::I32 {
                            self.line("i32.wrap_i64");
                        }
                    }
                    self.line(&format!("call ${}", name));
                    // Convert return values: if function returns i32(s), extend to i64
                    if fi.return_types.is_empty() {
                        if need_value {
                            self.line("i64.const 0");
                        }
                    } else if fi.return_types.len() == 1 {
                        if fi.return_types[0] == WasmType::I32 {
                            self.line("i64.extend_i32_s");
                        }
                        if !need_value {
                            self.line("drop");
                        }
                    } else if fi.return_types.len() == 2 {
                        // Multi-value return (e.g. llm, env_get, file_read) -> pack into single i64
                        // For now: drop second, extend first to i64
                        // TODO: handle multi-value returns properly
                        // Convention: the first value is the main result
                        if fi.return_types[0] == WasmType::I32 && fi.return_types[1] == WasmType::I32 {
                            // Two i32 results - combine into one i64 (low=first, high=second)
                            // But WASM multi-value is tricky. For now use a local.
                            // Actually for the current use cases, we just return the first value.
                            // Store second (on top of stack), keep first
                            self.line("drop"); // drop second result
                            self.line("i64.extend_i32_s");
                        }
                        if !need_value {
                            self.line("drop");
                        }
                    }
                } else {
                    for arg in args {
                        self.emit_expr(arg, true);
                    }
                    self.line(&format!("call ${}", name));
                    let returns_value = self.fn_has_return.get(name).copied().unwrap_or(true);
                    if returns_value && !need_value {
                        self.line("drop");
                    } else if !returns_value && need_value {
                        self.line("i64.const 0");
                    }
                }
            }
            IrExpr::CallIndirect(closure, args) => {
                // closure is a pointer to [i64 table_index][i64 captured_0]...
                // First push the closure pointer as first arg
                self.emit_expr(closure, true);
                // Then push other args
                for arg in args {
                    self.emit_expr(arg, true);
                }
                // Get table index from closure struct
                self.emit_expr(closure, true);
                self.line("i32.wrap_i64");
                self.line("i64.load");  // load table index (i64)
                self.line("i32.wrap_i64"); // convert to i32 for call_indirect
                let arity = args.len() + 1; // +1 for closure_ptr
                let type_name = format!("$closure_{}", args.len());
                self.line(&format!("call_indirect (type {})", type_name));
                if !need_value {
                    self.line("drop");
                }
            }
            IrExpr::BinOp(op, left, right) => {
                self.emit_expr(left, true);
                self.emit_expr(right, true);
                match op {
                    IrBinOp::Add => self.line("i64.add"),
                    IrBinOp::Sub => self.line("i64.sub"),
                    IrBinOp::Mul => self.line("i64.mul"),
                    IrBinOp::Div => self.line("i64.div_s"),
                    IrBinOp::Mod => self.line("i64.rem_s"),
                    IrBinOp::Eq => {
                        self.line("i64.eq");
                        self.line("i64.extend_i32_u");
                    }
                    IrBinOp::Neq => {
                        self.line("i64.ne");
                        self.line("i64.extend_i32_u");
                    }
                    IrBinOp::Lt => {
                        self.line("i64.lt_s");
                        self.line("i64.extend_i32_u");
                    }
                    IrBinOp::Gt => {
                        self.line("i64.gt_s");
                        self.line("i64.extend_i32_u");
                    }
                    IrBinOp::LtEq => {
                        self.line("i64.le_s");
                        self.line("i64.extend_i32_u");
                    }
                    IrBinOp::GtEq => {
                        self.line("i64.ge_s");
                        self.line("i64.extend_i32_u");
                    }
                }
                if !need_value {
                    self.line("drop");
                }
            }
            IrExpr::If(cond, then, else_) => {
                self.emit_expr(cond, true);
                // Truncate i64 condition to i32 for if
                self.line("i32.wrap_i64");
                if need_value && else_.is_some() {
                    self.push("if (result i64)");
                    self.emit_expr(then, true);
                    self.push("else");
                    self.emit_expr(else_.as_ref().unwrap(), true);
                    self.pop("end");
                } else if need_value {
                    // No else branch but need value - default to 0
                    self.push("if (result i64)");
                    self.emit_expr(then, true);
                    self.push("else");
                    self.line("i64.const 0");
                    self.pop("end");
                } else {
                    if else_.is_some() {
                        self.push("if");
                        self.emit_expr(then, false);
                        self.push("else");
                        self.emit_expr(else_.as_ref().unwrap(), false);
                        self.pop("end");
                    } else {
                        self.push("if");
                        self.emit_expr(then, false);
                        self.pop("end");
                    }
                }
            }
            IrExpr::Block(stmts, final_expr) => {
                for stmt in stmts {
                    match stmt {
                        IrStmt::Let(name, val) => {
                            self.emit_expr(val, true);
                            self.line(&format!("local.set ${}", name));
                        }
                        IrStmt::Expr(e) => {
                            self.emit_expr(e, false);
                        }
                    }
                }
                if let Some(final_expr) = final_expr {
                    self.emit_expr(final_expr, need_value);
                } else if need_value {
                    self.line("i64.const 0");
                }
            }
            IrExpr::Loop(label, body) => {
                let exit_label = format!("{}__exit", label);
                self.push(&format!("block ${}", exit_label));
                self.push(&format!("loop ${}", label));
                for stmt in body {
                    match stmt {
                        IrStmt::Let(name, val) => {
                            self.emit_expr(val, true);
                            self.line(&format!("local.set ${}", name));
                        }
                        IrStmt::Expr(e) => {
                            self.emit_expr(e, false);
                        }
                    }
                }
                self.pop("end"); // loop
                self.pop("end"); // block
                if need_value {
                    self.line("local.get $__tco_result");
                }
            }
            IrExpr::Break(label, val) => {
                self.emit_expr(val, true);
                self.line("local.set $__tco_result");
                self.line(&format!("br ${}", label));
            }
            IrExpr::Continue(label, updates) => {
                for (name, val) in updates {
                    self.emit_expr(val, true);
                    self.line(&format!("local.set ${}", name));
                }
                self.line(&format!("br ${}", label));
            }
            IrExpr::TaggedNew(tag, fields) => {
                self.emit_tagged_new(*tag, fields, need_value);
            }
            IrExpr::TaggedGetField(expr, index) => {
                // expr is a pointer to tagged union
                // Field at offset: 4 (tag) + 4 (field_count) + 8*index
                self.emit_expr(expr, true);
                self.line("i32.wrap_i64");
                let offset = 8 + 8 * index;
                self.line(&format!("i32.const {}", offset));
                self.line("i32.add");
                self.line("i64.load");
                if !need_value {
                    self.line("drop");
                }
            }
            IrExpr::TaggedGetTag(expr) => {
                self.emit_expr(expr, true);
                self.line("i32.wrap_i64");
                self.line("i32.load");
                self.line("i64.extend_i32_u");
                if !need_value {
                    self.line("drop");
                }
            }
            IrExpr::RecordNew(fields) => {
                self.emit_record_new(fields, need_value);
            }
            IrExpr::RecordGetField(expr, index) => {
                self.emit_expr(expr, true);
                self.line("i32.wrap_i64");
                let offset = 4 + 8 * index;
                self.line(&format!("i32.const {}", offset));
                self.line("i32.add");
                self.line("i64.load");
                if !need_value {
                    self.line("drop");
                }
            }
            IrExpr::ClosureNew(table_idx, captures) => {
                self.emit_closure_new(*table_idx, captures, need_value);
            }
            IrExpr::ClosureGetCapture(closure, index) => {
                self.emit_expr(closure, true);
                self.line("i32.wrap_i64");
                // Skip table_index (8 bytes), then index*8
                let offset = 8 + 8 * index;
                self.line(&format!("i32.const {}", offset));
                self.line("i32.add");
                self.line("i64.load");
                if !need_value {
                    self.line("drop");
                }
            }
            IrExpr::StringConcat(a, b) => {
                self.emit_expr(a, true);
                self.emit_expr(b, true);
                self.line("call $string_concat");
                if !need_value {
                    self.line("drop");
                }
            }
            IrExpr::Drop(expr) => {
                self.emit_expr(expr, true);
                self.line("drop");
            }
            IrExpr::ShowInt(expr) => {
                self.emit_expr(expr, true);
                self.line("call $show_int");
                if !need_value {
                    self.line("drop");
                }
            }
            IrExpr::ShowBool(expr) => {
                self.emit_expr(expr, true);
                self.line("call $show_bool");
                if !need_value {
                    self.line("drop");
                }
            }
            IrExpr::Spawn(closure) => {
                self.emit_expr(closure, true);
                self.line("call $spawn");
                if !need_value {
                    self.line("drop");
                }
            }
            IrExpr::Send(pid, msg) => {
                self.emit_expr(pid, true);
                self.emit_expr(msg, true);
                self.line("call $send");
                if need_value {
                    self.line("i64.const 0");
                }
            }
            IrExpr::Receive => {
                self.line("call $receive");
                if !need_value {
                    self.line("drop");
                }
            }
            IrExpr::SelfPid => {
                self.line("call $self_pid");
                if !need_value {
                    self.line("drop");
                }
            }
        }
    }

    fn alloc_scratch(&mut self) -> usize {
        let slot = self.scratch_slot;
        self.scratch_slot += 1;
        slot
    }

    fn free_scratch(&mut self) {
        self.scratch_slot -= 1;
    }

    fn scratch_addr(&self, slot: usize) -> u32 {
        // Scratch slots start at address 20, each 4 bytes
        20 + 4 * slot as u32
    }

    fn emit_tagged_new(&mut self, tag: u32, fields: &[IrExpr], need_value: bool) {
        let slot = self.alloc_scratch();
        let addr = self.scratch_addr(slot);
        let size = 4 + 4 + 8 * fields.len();

        self.line(&format!("i32.const {}", addr));
        self.line(&format!("i32.const {}", size));
        self.line("call $alloc");
        self.line("i32.store");

        // Store tag
        self.line(&format!("i32.const {}", addr));
        self.line("i32.load");
        self.line(&format!("i32.const {}", tag));
        self.line("i32.store");

        // Store field count
        self.line(&format!("i32.const {}", addr));
        self.line("i32.load");
        self.line("i32.const 4");
        self.line("i32.add");
        self.line(&format!("i32.const {}", fields.len()));
        self.line("i32.store");

        // Store each field
        for (i, field) in fields.iter().enumerate() {
            self.line(&format!("i32.const {}", addr));
            self.line("i32.load");
            let offset = 8 + 8 * i;
            self.line(&format!("i32.const {}", offset));
            self.line("i32.add");
            self.emit_expr(field, true);
            self.line("i64.store");
        }

        // Return pointer as i64
        self.line(&format!("i32.const {}", addr));
        self.line("i32.load");
        self.line("i64.extend_i32_u");
        if !need_value {
            self.line("drop");
        }
        self.free_scratch();
    }

    fn emit_record_new(&mut self, fields: &[(String, IrExpr)], need_value: bool) {
        let slot = self.alloc_scratch();
        let addr = self.scratch_addr(slot);
        let size = 4 + 8 * fields.len();

        self.line(&format!("i32.const {}", addr));
        self.line(&format!("i32.const {}", size));
        self.line("call $alloc");
        self.line("i32.store");

        // Store field count
        self.line(&format!("i32.const {}", addr));
        self.line("i32.load");
        self.line(&format!("i32.const {}", fields.len()));
        self.line("i32.store");

        // Store each field value
        for (i, (_, val)) in fields.iter().enumerate() {
            self.line(&format!("i32.const {}", addr));
            self.line("i32.load");
            let offset = 4 + 8 * i;
            self.line(&format!("i32.const {}", offset));
            self.line("i32.add");
            self.emit_expr(val, true);
            self.line("i64.store");
        }

        self.line(&format!("i32.const {}", addr));
        self.line("i32.load");
        self.line("i64.extend_i32_u");
        if !need_value {
            self.line("drop");
        }
        self.free_scratch();
    }

    fn emit_closure_new(&mut self, table_idx: u32, captures: &[IrExpr], need_value: bool) {
        let slot = self.alloc_scratch();
        let addr = self.scratch_addr(slot);
        let size = 8 + 8 * captures.len();

        self.line(&format!("i32.const {}", addr));
        self.line(&format!("i32.const {}", size));
        self.line("call $alloc");
        self.line("i32.store");

        // Store table index
        self.line(&format!("i32.const {}", addr));
        self.line("i32.load");
        self.line(&format!("i64.const {}", table_idx));
        self.line("i64.store");

        // Store captures
        for (i, cap) in captures.iter().enumerate() {
            self.line(&format!("i32.const {}", addr));
            self.line("i32.load");
            let offset = 8 + 8 * i;
            self.line(&format!("i32.const {}", offset));
            self.line("i32.add");
            self.emit_expr(cap, true);
            self.line("i64.store");
        }

        self.line(&format!("i32.const {}", addr));
        self.line("i32.load");
        self.line("i64.extend_i32_u");
        if !need_value {
            self.line("drop");
        }
        self.free_scratch();
    }
}
