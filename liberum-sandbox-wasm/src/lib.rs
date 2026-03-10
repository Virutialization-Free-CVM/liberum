// SPDX-FileCopyrightText: 2026 The Salus Contributors
//
// SPDX-License-Identifier: Apache-2.0

#![no_std]

use core::ops::Range;

use liberum_backend_salus::{BackendEvent, BackendRunToken, SalusSandboxBackend};
use liberum_core::{
    ComponentDescriptor, ComponentType, ConfidentialExecutionContext, DemoLogEvent,
    ForcedExitReason, LiberumMonitor, Result, SandboxType, SHA384_DIGEST_BYTES,
};

pub const ENTRY_SHIM_COMPONENT_ID: u64 = 1;
pub const RUNTIME_CORE_COMPONENT_ID: u64 = 2;
pub const GUEST_IMAGE_COMPONENT_ID: u64 = 3;
pub const EXIT_SHIM_COMPONENT_ID: u64 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WasmSandboxRunState {
    pub ctx: ConfidentialExecutionContext,
    pub runtime_token: BackendRunToken,
    pub guest_token: BackendRunToken,
}

pub struct WasmSandbox {
    monitor: LiberumMonitor,
    backend: SalusSandboxBackend,
}

impl WasmSandbox {
    pub fn new() -> Self {
        Self {
            monitor: LiberumMonitor::new(),
            backend: SalusSandboxBackend::new(),
        }
    }

    pub fn install_demo_components(&mut self) -> Result<()> {
        self.monitor
            .create_component(Self::entry_shim_descriptor())?;
        self.monitor
            .create_component(Self::runtime_core_descriptor())?;
        self.monitor
            .create_component(Self::guest_image_descriptor())?;
        self.monitor
            .create_component(Self::exit_shim_descriptor())?;
        Ok(())
    }

    pub fn load_component(&mut self, component_id: u64, bytes_loaded: usize) -> Result<()> {
        self.monitor.load_component(component_id, bytes_loaded)?;
        self.backend.stage_component(component_id, bytes_loaded);
        Ok(())
    }

    pub fn finalize_component(
        &mut self,
        component_id: u64,
        measurement: [u8; SHA384_DIGEST_BYTES],
    ) -> Result<()> {
        self.monitor.finalize_component(component_id, measurement)?;
        self.backend.finalize_component(component_id, measurement);
        Ok(())
    }

    pub fn register_shims(&mut self) -> Result<()> {
        self.monitor.register_entry_shim(ENTRY_SHIM_COMPONENT_ID)?;
        self.monitor.register_exit_shim(EXIT_SHIM_COMPONENT_ID)?;
        Ok(())
    }

    pub fn authorize_demo_path(&mut self) -> Result<()> {
        let runtime_entry = self.monitor.descriptor(RUNTIME_CORE_COMPONENT_ID)?.entry_pc;
        let guest_entry = self.monitor.descriptor(GUEST_IMAGE_COMPONENT_ID)?.entry_pc;
        self.monitor.authorize_transition(
            ENTRY_SHIM_COMPONENT_ID,
            RUNTIME_CORE_COMPONENT_ID,
            runtime_entry,
        )?;
        self.monitor.authorize_transition(
            RUNTIME_CORE_COMPONENT_ID,
            GUEST_IMAGE_COMPONENT_ID,
            guest_entry,
        )?;
        Ok(())
    }

    pub fn enter_demo(&mut self) -> Result<WasmSandboxRunState> {
        let runtime_desc = self.monitor.descriptor(RUNTIME_CORE_COMPONENT_ID)?.clone();
        let runtime_measurement = self.monitor.measurement(RUNTIME_CORE_COMPONENT_ID)?;
        self.monitor.verify_jmp(
            ENTRY_SHIM_COMPONENT_ID,
            RUNTIME_CORE_COMPONENT_ID,
            runtime_desc.entry_pc,
            runtime_measurement,
        )?;
        let ctx = self.monitor.begin_confidential_run(RUNTIME_CORE_COMPONENT_ID)?;
        let runtime_token = self
            .backend
            .enter_component(RUNTIME_CORE_COMPONENT_ID, runtime_desc.entry_pc);

        let guest_desc = self.monitor.descriptor(GUEST_IMAGE_COMPONENT_ID)?.clone();
        let guest_measurement = self.monitor.measurement(GUEST_IMAGE_COMPONENT_ID)?;
        self.monitor.verify_jmp(
            RUNTIME_CORE_COMPONENT_ID,
            GUEST_IMAGE_COMPONENT_ID,
            guest_desc.entry_pc,
            guest_measurement,
        )?;
        let guest_token = self
            .backend
            .enter_component(GUEST_IMAGE_COMPONENT_ID, guest_desc.entry_pc);

        Ok(WasmSandboxRunState {
            ctx,
            runtime_token,
            guest_token,
        })
    }

    pub fn force_demo_exit(
        &mut self,
        state: &mut WasmSandboxRunState,
        reason: ForcedExitReason,
    ) -> Result<u64> {
        let exit_pc = self.monitor.force_trusted_exit(&mut state.ctx, reason)?;
        self.backend
            .trusted_exit(state.ctx.active_component_id, reason, exit_pc);
        Ok(exit_pc)
    }

    pub fn monitor_log(&self) -> &[DemoLogEvent] {
        self.monitor.log()
    }

    pub fn backend_events(&self) -> &[BackendEvent] {
        self.backend.events()
    }

    fn entry_shim_descriptor() -> ComponentDescriptor {
        Self::component_descriptor(
            ENTRY_SHIM_COMPONENT_ID,
            ComponentType::EntryShim,
            0x1000,
            Some(0x1000),
            0x1000..0x1100,
        )
    }

    fn runtime_core_descriptor() -> ComponentDescriptor {
        Self::component_descriptor(
            RUNTIME_CORE_COMPONENT_ID,
            ComponentType::RuntimeCore,
            0x2000,
            None,
            0x2000..0x3000,
        )
    }

    fn guest_image_descriptor() -> ComponentDescriptor {
        Self::component_descriptor(
            GUEST_IMAGE_COMPONENT_ID,
            ComponentType::GuestImage,
            0x3000,
            None,
            0x3000..0x4000,
        )
    }

    fn exit_shim_descriptor() -> ComponentDescriptor {
        Self::component_descriptor(
            EXIT_SHIM_COMPONENT_ID,
            ComponentType::ExitShim,
            0x4000,
            Some(0x4000),
            0x4000..0x4100,
        )
    }

    fn component_descriptor(
        component_id: u64,
        component_type: ComponentType,
        entry_pc: u64,
        exit_pc: Option<u64>,
        code_region: Range<u64>,
    ) -> ComponentDescriptor {
        ComponentDescriptor {
            component_id,
            component_type,
            sandbox_type: SandboxType::Wasm,
            entry_pc,
            exit_pc,
            code_region,
            finalized: false,
            measurement: [0; SHA384_DIGEST_BYTES],
        }
    }
}
