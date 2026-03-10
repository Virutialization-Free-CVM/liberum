// SPDX-FileCopyrightText: 2026 The Salus Contributors
//
// SPDX-License-Identifier: Apache-2.0

#![no_std]

use device_tree::Fdt;
use liberum_core::{
    ComponentDescriptor, ComponentType, DemoLogEvent, ForcedExitReason, LiberumMonitor,
    SandboxType, SHA384_DIGEST_BYTES,
};
use riscv_regs::{GprIndex, Readable, Trap, CSR};
use s_mode_utils::abort::abort;
use s_mode_utils::{print::*, sbi_console::SbiConsole};
use sbi_rs::api::{base, cove_host, nacl, reset};
use sbi_rs::{SbiMessage, EXT_COVE_HOST};
use test_workloads::consts::*;

const NUM_COVE_PTE_PAGES: u64 = 10;
const NUM_WASMRT_GUEST_IMAGE_PAGES: u64 = 8;
const NUM_WASMRT_GUEST_ZERO_PAGES: u64 = 64;
const NUM_WASMRT_HOST_PAGES: u64 = 512;
const ENTRY_SHIM_PC: u64 = USABLE_RAM_START_ADDRESS;
const RUNTIME_CORE_PC: u64 = USABLE_RAM_START_ADDRESS;
const EXIT_SHIM_PC: u64 = USABLE_RAM_START_ADDRESS;
const DEMO_NONCE: u64 = 0x1357_2468;
const WASM_RESULT: u64 = 62;
const SANDBOX_SECRET: u64 = 0xc0de_c0de;
const ENTRY_SHIM_COMPONENT_ID: u64 = 1;
const RUNTIME_CORE_COMPONENT_ID: u64 = 2;
const GUEST_IMAGE_COMPONENT_ID: u64 = 3;
const EXIT_SHIM_COMPONENT_ID: u64 = 4;
const ENTRY_SHIM_MEASUREMENT: [u8; SHA384_DIGEST_BYTES] = [0x11; SHA384_DIGEST_BYTES];
const RUNTIME_CORE_MEASUREMENT: [u8; SHA384_DIGEST_BYTES] = [0x22; SHA384_DIGEST_BYTES];
const GUEST_IMAGE_MEASUREMENT: [u8; SHA384_DIGEST_BYTES] = [0x33; SHA384_DIGEST_BYTES];
const EXIT_SHIM_MEASUREMENT: [u8; SHA384_DIGEST_BYTES] = [0x44; SHA384_DIGEST_BYTES];

fn fence_memory() {
    cove_host::tsm_initiate_fence().expect("tellus_wasmrt: TsmInitiateFence failed");
}

unsafe fn convert_pages(addr: u64, num_pages: u64) {
    cove_host::convert_pages(addr, num_pages).expect("tellus_wasmrt: TsmConvertPages failed");
    fence_memory();
}

fn reclaim_pages(addr: u64, num_pages: u64) {
    cove_host::reclaim_pages(addr, num_pages).expect("tellus_wasmrt: TsmReclaimPages failed");
}

fn component_descriptor(
    component_id: u64,
    component_type: ComponentType,
    entry_pc: u64,
    exit_pc: Option<u64>,
    code_region_end: u64,
) -> ComponentDescriptor {
    ComponentDescriptor {
        component_id,
        component_type,
        sandbox_type: SandboxType::Wasm,
        entry_pc,
        exit_pc,
        code_region: entry_pc..code_region_end,
        finalized: false,
        measurement: [0; SHA384_DIGEST_BYTES],
    }
}

fn print_monitor_event(event: &DemoLogEvent) {
    match event {
        DemoLogEvent::ComponentCreated(_, ty) => println!("Liberum: create_component({:?})", ty),
        DemoLogEvent::ComponentLoaded(id) => println!("Liberum: load_component(id={id})"),
        DemoLogEvent::ComponentFinalized(id) => println!("Liberum: finalize_component(id={id})"),
        DemoLogEvent::EntryShimRegistered(id) => {
            println!("Liberum: register_entry_shim(component_id={id})")
        }
        DemoLogEvent::ExitShimRegistered(id) => {
            println!("Liberum: register_exit_shim(component_id={id})")
        }
        DemoLogEvent::VerifyJmpAccepted(id, pc) => {
            println!("Liberum: verify_jmp accepted component_id={id} target_pc=0x{pc:x}")
        }
        DemoLogEvent::VerifyJmpRejected(id, pc, err) => {
            println!(
                "Liberum: verify_jmp rejected component_id={id} target_pc=0x{pc:x} reason={:?}",
                err
            )
        }
        DemoLogEvent::EnteredConfidential(id) => {
            println!("Liberum: enter confidential component_id={id}")
        }
        DemoLogEvent::ForcedTrustedExit(id, reason, exit_pc) => {
            println!(
                "Liberum: trusted exit component_id={id} reason={:?} exit_pc=0x{exit_pc:x}",
                reason
            )
        }
    }
}

fn print_last_monitor_event(monitor: &LiberumMonitor) {
    if let Some(event) = monitor.log().last() {
        print_monitor_event(event);
    }
}

pub fn run_tellus_wasmrt_host(console_mem: &'static mut [u8], fdt_addr: u64) -> ! {
    SbiConsole::set_as_console(console_mem);

    println!("Tellus WasmRT: boot");
    let mut monitor = LiberumMonitor::new();
    monitor
        .create_component(component_descriptor(
            ENTRY_SHIM_COMPONENT_ID,
            ComponentType::EntryShim,
            ENTRY_SHIM_PC,
            Some(EXIT_SHIM_PC),
            ENTRY_SHIM_PC + PAGE_SIZE_4K,
        ))
        .unwrap();
    print_last_monitor_event(&monitor);
    monitor
        .create_component(component_descriptor(
            RUNTIME_CORE_COMPONENT_ID,
            ComponentType::RuntimeCore,
            RUNTIME_CORE_PC,
            None,
            RUNTIME_CORE_PC + PAGE_SIZE_4K * NUM_WASMRT_GUEST_IMAGE_PAGES,
        ))
        .unwrap();
    print_last_monitor_event(&monitor);
    monitor
        .create_component(component_descriptor(
            GUEST_IMAGE_COMPONENT_ID,
            ComponentType::GuestImage,
            USABLE_RAM_START_ADDRESS,
            None,
            USABLE_RAM_START_ADDRESS + PAGE_SIZE_4K * NUM_WASMRT_GUEST_IMAGE_PAGES,
        ))
        .unwrap();
    print_last_monitor_event(&monitor);
    monitor
        .create_component(component_descriptor(
            EXIT_SHIM_COMPONENT_ID,
            ComponentType::ExitShim,
            EXIT_SHIM_PC,
            Some(EXIT_SHIM_PC),
            EXIT_SHIM_PC + PAGE_SIZE_4K,
        ))
        .unwrap();
    print_last_monitor_event(&monitor);
    monitor.register_entry_shim(ENTRY_SHIM_COMPONENT_ID).unwrap();
    print_last_monitor_event(&monitor);
    monitor.register_exit_shim(EXIT_SHIM_COMPONENT_ID).unwrap();
    print_last_monitor_event(&monitor);
    monitor.load_component(ENTRY_SHIM_COMPONENT_ID, 0).unwrap();
    print_last_monitor_event(&monitor);
    monitor.load_component(EXIT_SHIM_COMPONENT_ID, 0).unwrap();
    print_last_monitor_event(&monitor);

    let fdt = unsafe { Fdt::new_from_raw_pointer(fdt_addr as *const u8) }
        .expect("tellus_wasmrt: invalid FDT");
    let mem_range = fdt.memory_regions().next().expect("tellus_wasmrt: no memory");
    let mut next_page = (mem_range.base() + mem_range.size() / 2) & !0x3fff;

    base::probe_sbi_extension(EXT_COVE_HOST).expect("tellus_wasmrt: COVE host missing");
    let tsm_info = cove_host::get_info().expect("tellus_wasmrt: TsmGetInfo failed");

    let tvm_create_pages = 4 + tsm_info.tvm_state_pages;
    let state_pages_base = next_page;
    unsafe { convert_pages(next_page, tvm_create_pages) };
    let tvm_page_directory_addr = state_pages_base;
    let tvm_state_addr = tvm_page_directory_addr + 4 * PAGE_SIZE_4K;
    let vmid = cove_host::tvm_create(tvm_page_directory_addr, tvm_state_addr)
        .expect("tellus_wasmrt: TvmCreate failed");
    next_page += PAGE_SIZE_4K * tvm_create_pages;

    let num_shmem_pages =
        (core::mem::size_of::<sbi_rs::NaclShmem>() as u64).div_ceil(PAGE_SIZE_4K);
    let shmem_ptr = next_page as *mut sbi_rs::NaclShmem;
    next_page += num_shmem_pages * PAGE_SIZE_4K;
    unsafe { nacl::register_shmem(shmem_ptr).expect("tellus_wasmrt: register_shmem failed") };
    let shmem = unsafe { cove_host::TsmShmemAreaRef::new(shmem_ptr) };

    unsafe { convert_pages(next_page, NUM_COVE_PTE_PAGES) };
    cove_host::add_page_table_pages(vmid, next_page, NUM_COVE_PTE_PAGES)
        .expect("tellus_wasmrt: AddPageTablePages failed");
    next_page += PAGE_SIZE_4K * NUM_COVE_PTE_PAGES;

    let vcpu_pages_base = next_page;
    unsafe { convert_pages(vcpu_pages_base, tsm_info.tvm_vcpu_state_pages) };
    next_page += PAGE_SIZE_4K * tsm_info.tvm_vcpu_state_pages;
    cove_host::add_vcpu(vmid, 0, vcpu_pages_base).expect("tellus_wasmrt: TvmCpuCreate failed");

    let guest_image_base = USABLE_RAM_START_ADDRESS + PAGE_SIZE_4K * NUM_WASMRT_HOST_PAGES;
    let guest_image = unsafe {
        core::slice::from_raw_parts(
            guest_image_base as *const u8,
            (PAGE_SIZE_4K * NUM_WASMRT_GUEST_IMAGE_PAGES) as usize,
        )
    };
    let guest_pages_base = next_page;
    monitor
        .load_component(
            RUNTIME_CORE_COMPONENT_ID,
            (NUM_WASMRT_GUEST_IMAGE_PAGES * PAGE_SIZE_4K) as usize,
        )
        .unwrap();
    print_last_monitor_event(&monitor);
    monitor
        .load_component(
            GUEST_IMAGE_COMPONENT_ID,
            (NUM_WASMRT_GUEST_IMAGE_PAGES * PAGE_SIZE_4K) as usize,
        )
        .unwrap();
    print_last_monitor_event(&monitor);

    println!("Liberum: host nonce = 0x{DEMO_NONCE:x}");
    println!(
        "Liberum: expected sandbox response = 0x{:x}",
        SANDBOX_SECRET ^ DEMO_NONCE ^ WASM_RESULT
    );

    cove_host::add_memory_region(
        vmid,
        USABLE_RAM_START_ADDRESS,
        (NUM_WASMRT_GUEST_IMAGE_PAGES + NUM_WASMRT_GUEST_ZERO_PAGES) * PAGE_SIZE_4K,
    )
    .expect("tellus_wasmrt: TvmAddMemoryRegion failed");

    unsafe { convert_pages(next_page, NUM_WASMRT_GUEST_IMAGE_PAGES + NUM_WASMRT_GUEST_ZERO_PAGES) };
    cove_host::add_measured_pages(
        vmid,
        guest_image,
        next_page,
        sbi_rs::TsmPageType::Page4k,
        USABLE_RAM_START_ADDRESS,
    )
    .expect("tellus_wasmrt: TvmAddMeasuredPages failed");
    let zero_pages_base = next_page + PAGE_SIZE_4K * NUM_WASMRT_GUEST_IMAGE_PAGES;
    cove_host::tvm_finalize(vmid, USABLE_RAM_START_ADDRESS, DEMO_NONCE)
        .expect("tellus_wasmrt: Finalize failed");
    cove_host::add_zero_pages(
        vmid,
        zero_pages_base,
        sbi_rs::TsmPageType::Page4k,
        NUM_WASMRT_GUEST_ZERO_PAGES,
        USABLE_RAM_START_ADDRESS + PAGE_SIZE_4K * NUM_WASMRT_GUEST_IMAGE_PAGES,
    )
    .expect("tellus_wasmrt: TvmAddZeroPages failed");

    monitor
        .finalize_component(ENTRY_SHIM_COMPONENT_ID, ENTRY_SHIM_MEASUREMENT)
        .unwrap();
    print_last_monitor_event(&monitor);
    monitor
        .finalize_component(RUNTIME_CORE_COMPONENT_ID, RUNTIME_CORE_MEASUREMENT)
        .unwrap();
    print_last_monitor_event(&monitor);
    monitor
        .finalize_component(GUEST_IMAGE_COMPONENT_ID, GUEST_IMAGE_MEASUREMENT)
        .unwrap();
    print_last_monitor_event(&monitor);
    monitor
        .finalize_component(EXIT_SHIM_COMPONENT_ID, EXIT_SHIM_MEASUREMENT)
        .unwrap();
    print_last_monitor_event(&monitor);
    monitor
        .authorize_transition(
            ENTRY_SHIM_COMPONENT_ID,
            RUNTIME_CORE_COMPONENT_ID,
            RUNTIME_CORE_PC,
        )
        .unwrap();
    monitor
        .authorize_transition(
            RUNTIME_CORE_COMPONENT_ID,
            GUEST_IMAGE_COMPONENT_ID,
            USABLE_RAM_START_ADDRESS,
        )
        .unwrap();
    monitor
        .verify_jmp(
            ENTRY_SHIM_COMPONENT_ID,
            RUNTIME_CORE_COMPONENT_ID,
            RUNTIME_CORE_PC,
            RUNTIME_CORE_MEASUREMENT,
        )
        .unwrap();
    print_last_monitor_event(&monitor);
    let mut exec_ctx = monitor.begin_confidential_run(RUNTIME_CORE_COMPONENT_ID).unwrap();
    print_last_monitor_event(&monitor);
    monitor
        .verify_jmp(
            RUNTIME_CORE_COMPONENT_ID,
            GUEST_IMAGE_COMPONENT_ID,
            USABLE_RAM_START_ADDRESS,
            GUEST_IMAGE_MEASUREMENT,
        )
        .unwrap();
    print_last_monitor_event(&monitor);

    loop {
        let fatal = cove_host::tvm_run(vmid, 0).expect("tellus_wasmrt: TvmCpuRun failed") != 0;
        let scause = CSR.scause.get();
        match Trap::from_scause(scause).expect("tellus_wasmrt: invalid scause") {
            Trap::Exception(riscv_regs::Exception::VirtualSupervisorEnvCall) => {
                let mut a_regs = [0u64; 8];
                for (i, reg) in a_regs.iter_mut().enumerate() {
                    *reg = shmem.gpr(GprIndex::A0 as usize + i);
                }

                match SbiMessage::from_regs(&a_regs) {
                    Ok(SbiMessage::PutChar(c)) => {
                        print!("{}", c as u8 as char);
                        shmem.set_gpr(GprIndex::A0 as usize, 0);
                    }
                    Ok(SbiMessage::Reset(_)) => {
                        monitor
                            .force_trusted_exit(&mut exec_ctx, ForcedExitReason::SyntheticTrap)
                            .unwrap();
                        print_last_monitor_event(&monitor);
                        break;
                    }
                    Ok(other) => {
                        println!("Liberum: unexpected guest ECALL {:?}", other);
                        monitor
                            .force_trusted_exit(&mut exec_ctx, ForcedExitReason::SyntheticTrap)
                            .unwrap();
                        print_last_monitor_event(&monitor);
                        break;
                    }
                    Err(err) => {
                        println!("Liberum: bad SBI message {:?}", err);
                        monitor
                            .force_trusted_exit(&mut exec_ctx, ForcedExitReason::SyntheticTrap)
                            .unwrap();
                        print_last_monitor_event(&monitor);
                        break;
                    }
                }
            }
            Trap::Interrupt(i) => {
                println!("Liberum: interrupt {:?}", i);
                monitor
                    .force_trusted_exit(&mut exec_ctx, ForcedExitReason::SyntheticInterrupt)
                    .unwrap();
                print_last_monitor_event(&monitor);
                break;
            }
            Trap::Exception(e) if !fatal => {
                println!("Liberum: exception {:?}", e);
                monitor
                    .force_trusted_exit(&mut exec_ctx, ForcedExitReason::SyntheticTrap)
                    .unwrap();
                print_last_monitor_event(&monitor);
                break;
            }
            Trap::Exception(e) => {
                println!("Liberum: fatal exception {:?}", e);
                monitor
                    .force_trusted_exit(&mut exec_ctx, ForcedExitReason::SyntheticTrap)
                    .unwrap();
                print_last_monitor_event(&monitor);
                break;
            }
        }
    }

    cove_host::tvm_destroy(vmid).expect("tellus_wasmrt: TvmDestroy failed");
    reclaim_pages(
        guest_pages_base,
        NUM_WASMRT_GUEST_IMAGE_PAGES + NUM_WASMRT_GUEST_ZERO_PAGES,
    );
    reclaim_pages(state_pages_base, tvm_create_pages);
    reclaim_pages(vcpu_pages_base, tsm_info.tvm_vcpu_state_pages);
    nacl::unregister_shmem().expect("tellus_wasmrt: unregister_shmem failed");

    println!("Tellus WasmRT: all ok");
    reset::shutdown().expect("tellus_wasmrt: shutdown failed");
    abort()
}
