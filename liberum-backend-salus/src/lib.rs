// SPDX-FileCopyrightText: 2026 The Salus Contributors
//
// SPDX-License-Identifier: Apache-2.0

#![no_std]

extern crate alloc;

use alloc::vec::Vec;

use liberum_core::{ForcedExitReason, SHA384_DIGEST_BYTES};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendEvent {
    ComponentStaged(u64, usize),
    ComponentFinalized(u64, [u8; SHA384_DIGEST_BYTES]),
    ComponentEntered(u64, u64),
    TrustedExit(u64, ForcedExitReason, u64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendRunToken {
    pub component_id: u64,
    pub entry_pc: u64,
}

#[derive(Debug)]
pub struct SalusSandboxBackend {
    events: Vec<BackendEvent>,
}

impl SalusSandboxBackend {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn stage_component(&mut self, component_id: u64, bytes_loaded: usize) {
        self.events
            .push(BackendEvent::ComponentStaged(component_id, bytes_loaded));
    }

    pub fn finalize_component(
        &mut self,
        component_id: u64,
        measurement: [u8; SHA384_DIGEST_BYTES],
    ) {
        self.events
            .push(BackendEvent::ComponentFinalized(component_id, measurement));
    }

    pub fn enter_component(&mut self, component_id: u64, entry_pc: u64) -> BackendRunToken {
        self.events
            .push(BackendEvent::ComponentEntered(component_id, entry_pc));
        BackendRunToken {
            component_id,
            entry_pc,
        }
    }

    pub fn trusted_exit(
        &mut self,
        component_id: u64,
        reason: ForcedExitReason,
        exit_pc: u64,
    ) {
        self.events
            .push(BackendEvent::TrustedExit(component_id, reason, exit_pc));
    }

    pub fn events(&self) -> &[BackendEvent] {
        &self.events
    }
}
