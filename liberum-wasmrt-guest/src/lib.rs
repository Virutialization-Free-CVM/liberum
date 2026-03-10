// SPDX-FileCopyrightText: 2026 The Salus Contributors
//
// SPDX-License-Identifier: Apache-2.0

#![no_std]

use s_mode_utils::abort::abort;
use sbi_rs::api::{base, reset};
use sbi_rs::{ecall_send, SbiMessage};

const WASM_MAGIC: &[u8; 4] = b"\0asm";
const WASM_VERSION: &[u8; 4] = &[1, 0, 0, 0];
const SANDBOX_SECRET: u32 = 0xc0de_c0de;
const WASM_MODULE: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d,
    0x01, 0x00, 0x00, 0x00,
    0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f,
    0x03, 0x02, 0x01, 0x00,
    0x07, 0x07, 0x01, 0x03, 0x72, 0x75, 0x6e, 0x00, 0x00,
    0x0a, 0x09, 0x01, 0x07, 0x00, 0x41, 0x14, 0x41, 0x2a, 0x6a, 0x0b,
];

#[derive(Clone, Copy)]
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    fn read_u8(&mut self) -> u8 {
        let value = *self.bytes.get(self.offset).expect("wasmrt_guest: truncated module");
        self.offset += 1;
        value
    }

    fn read_bytes(&mut self, len: usize) -> &'a [u8] {
        let end = self.offset.checked_add(len).expect("wasmrt_guest: overflow");
        let slice = self.bytes.get(self.offset..end).expect("wasmrt_guest: truncated slice");
        self.offset = end;
        slice
    }

    fn read_leb_u32(&mut self) -> u32 {
        let mut shift = 0;
        let mut result = 0u32;
        loop {
            let byte = self.read_u8();
            result |= ((byte & 0x7f) as u32) << shift;
            if byte & 0x80 == 0 {
                return result;
            }
            shift += 7;
            assert!(shift < 32, "wasmrt_guest: invalid LEB");
        }
    }

    fn read_leb_i32(&mut self) -> i32 {
        let byte = self.read_u8();
        assert!(byte & 0x80 == 0, "wasmrt_guest: multi-byte const unsupported");
        (byte as i8) as i32
    }
}

fn putchar(byte: u8) {
    let msg = SbiMessage::PutChar(byte as u64);
    unsafe { ecall_send::<()>(&msg) }.expect("wasmrt_guest: PutChar failed");
}

fn puts(message: &str) {
    for byte in message.as_bytes() {
        putchar(*byte);
    }
}

fn put_u32(mut value: u32) {
    if value == 0 {
        putchar(b'0');
        return;
    }

    let mut digits = [0u8; 10];
    let mut len = 0;
    while value != 0 {
        digits[len] = (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    while len != 0 {
        len -= 1;
        putchar(b'0' + digits[len]);
    }
}

fn put_hex_u32(value: u32) {
    puts("0x");
    for shift in (0..8).rev() {
        let nibble = ((value >> (shift * 4)) & 0xf) as u8;
        let byte = match nibble {
            0..=9 => b'0' + nibble,
            _ => b'a' + (nibble - 10),
        };
        putchar(byte);
    }
}

fn execute_wasm_run(module: &[u8]) -> u32 {
    let mut reader = Reader::new(module);
    assert_eq!(reader.read_bytes(4), WASM_MAGIC, "wasmrt_guest: bad magic");
    assert_eq!(reader.read_bytes(4), WASM_VERSION, "wasmrt_guest: bad version");

    let mut code_body = None;
    let mut saw_run_export = false;

    while reader.remaining() != 0 {
        let section_id = reader.read_u8();
        let section_len = reader.read_leb_u32() as usize;
        let section_bytes = reader.read_bytes(section_len);
        let mut section = Reader::new(section_bytes);

        match section_id {
            1 => {
                let type_count = section.read_leb_u32();
                assert_eq!(type_count, 1, "wasmrt_guest: unexpected type count");
                assert_eq!(section.read_u8(), 0x60, "wasmrt_guest: expected func type");
                assert_eq!(section.read_leb_u32(), 0, "wasmrt_guest: params unsupported");
                assert_eq!(section.read_leb_u32(), 1, "wasmrt_guest: expected result");
                assert_eq!(section.read_u8(), 0x7f, "wasmrt_guest: expected i32 result");
            }
            3 => {
                let function_count = section.read_leb_u32();
                assert_eq!(function_count, 1, "wasmrt_guest: expected one function");
                assert_eq!(section.read_leb_u32(), 0, "wasmrt_guest: bad type index");
            }
            7 => {
                let export_count = section.read_leb_u32();
                assert_eq!(export_count, 1, "wasmrt_guest: expected one export");
                let name_len = section.read_leb_u32() as usize;
                let name = section.read_bytes(name_len);
                let kind = section.read_u8();
                let index = section.read_leb_u32();
                saw_run_export = name == b"run" && kind == 0 && index == 0;
            }
            10 => {
                let body_count = section.read_leb_u32();
                assert_eq!(body_count, 1, "wasmrt_guest: expected one body");
                let body_len = section.read_leb_u32() as usize;
                code_body = Some(section.read_bytes(body_len));
            }
            _ => {}
        }
    }

    assert!(saw_run_export, "wasmrt_guest: missing run export");
    let mut body = Reader::new(code_body.expect("wasmrt_guest: missing code body"));
    assert_eq!(body.read_leb_u32(), 0, "wasmrt_guest: locals unsupported");

    let mut stack = [0i32; 8];
    let mut stack_len = 0usize;
    loop {
        match body.read_u8() {
            0x41 => {
                stack[stack_len] = body.read_leb_i32();
                stack_len += 1;
            }
            0x6a => {
                assert!(stack_len >= 2, "wasmrt_guest: stack underflow");
                let rhs = stack[stack_len - 1];
                let lhs = stack[stack_len - 2];
                stack[stack_len - 2] = lhs.wrapping_add(rhs);
                stack_len -= 1;
            }
            0x0b => {
                assert_eq!(stack_len, 1, "wasmrt_guest: bad stack state");
                return stack[0] as u32;
            }
            opcode => panic!("wasmrt_guest: unsupported opcode 0x{:x}", opcode),
        }
    }
}

pub fn run_wasmrt_guest(boot_args: u64) -> ! {
    base::probe_sbi_extension(sbi_rs::EXT_COVE_GUEST).expect("wasmrt_guest: COVE guest missing");

    puts("wasmrt_guest: entry_shim -> runtime_core -> guest_image\n");
    puts("wasmrt_guest: embedded wasm module export run() -> i32\n");
    let result = execute_wasm_run(WASM_MODULE);
    let response = SANDBOX_SECRET ^ (boot_args as u32) ^ result;
    puts("wasmrt_guest: host nonce ");
    put_hex_u32(boot_args as u32);
    puts("\n");
    puts("wasmrt_guest: wasm run() returned ");
    put_u32(result);
    puts("\n");
    puts("wasmrt_guest: sandbox response ");
    put_hex_u32(response);
    puts("\n");
    puts("wasmrt_guest: done\n");

    reset::reset(sbi_rs::ResetType::Shutdown, sbi_rs::ResetReason::NoReason)
        .expect("wasmrt_guest: shutdown failed");
    unreachable!();
}

pub fn secondary_guest_init() -> ! {
    abort()
}
