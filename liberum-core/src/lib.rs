// SPDX-FileCopyrightText: 2026 The Salus Contributors
//
// SPDX-License-Identifier: Apache-2.0

#![no_std]

use arrayvec::ArrayVec;
use core::ops::Range;

pub const SHA384_DIGEST_BYTES: usize = 48;
pub const MAX_COMPONENTS: usize = 8;
pub const MAX_TRANSITIONS: usize = 8;
pub const MAX_LOG_EVENTS: usize = 32;

// Liberum manages measured sandbox components instead of VM/vCPU objects.
// This crate holds the minimal component model and policy checks used by the
// current PoC.

// Minimal component roles used in the first Wasm-centric demo path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentType {
    RuntimeCore,
    GuestImage,
    EntryShim,
    ExitShim,
}

// Small lifecycle for deadline-one: create, load, finalize, run, exit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentState {
    Created,
    Loaded,
    Finalized,
    Running,
    Exited,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SandboxType {
    Wasm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForcedExitReason {
    SyntheticInterrupt,
    SyntheticTrap,
    MeasurementMismatch,
    UnauthorizedEntry,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiberumError {
    DuplicateComponentId,
    ComponentNotFound,
    ComponentNotLoaded,
    ComponentNotFinalized,
    TransitionNotAuthorized,
    EntryPointMismatch,
    ExitShimMissing,
    EntryShimMissing,
    TargetOutsideCodeRegion,
    MeasurementMismatch,
    InvalidState,
    CapacityExceeded,
}

pub type Result<T> = core::result::Result<T, LiberumError>;

// Frozen metadata that verify_jmp and trusted-exit policy consult.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentDescriptor {
    pub component_id: u64,
    pub component_type: ComponentType,
    pub sandbox_type: SandboxType,
    pub entry_pc: u64,
    pub exit_pc: Option<u64>,
    pub code_region: Range<u64>,
    pub finalized: bool,
    pub measurement: [u8; SHA384_DIGEST_BYTES],
}

impl ComponentDescriptor {
    pub fn contains(&self, pc: u64) -> bool {
        self.code_region.start <= pc && pc < self.code_region.end
    }
}

// Allowed edge in the component transition graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthorizedTransition {
    pub from_component_id: u64,
    pub to_component_id: u64,
    pub target_pc: u64,
}

// Minimal execution context tracked while a confidential component is active.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConfidentialExecutionContext {
    pub active_component_id: u64,
    pub sandbox_active: bool,
    pub trusted_exit_pending: bool,
    pub last_exit_reason: Option<ForcedExitReason>,
}

// Demo-visible events used to show the policy path in logs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemoLogEvent {
    ComponentCreated(u64, ComponentType),
    ComponentLoaded(u64),
    ComponentFinalized(u64),
    EntryShimRegistered(u64),
    ExitShimRegistered(u64),
    VerifyJmpAccepted(u64, u64),
    VerifyJmpRejected(u64, u64, LiberumError),
    EnteredConfidential(u64),
    ForcedTrustedExit(u64, ForcedExitReason, u64),
}

#[derive(Clone, Debug)]
struct ManagedComponent {
    descriptor: ComponentDescriptor,
    state: ComponentState,
    bytes_loaded: usize,
}

// In-memory monitor for the current PoC. It owns component metadata, the
// authorized transition graph, and a small log of observable policy events.
pub struct LiberumMonitor {
    components: ArrayVec<ManagedComponent, MAX_COMPONENTS>,
    transitions: ArrayVec<AuthorizedTransition, MAX_TRANSITIONS>,
    entry_shim_component_id: Option<u64>,
    exit_shim_component_id: Option<u64>,
    log: ArrayVec<DemoLogEvent, MAX_LOG_EVENTS>,
}

impl LiberumMonitor {
    pub fn new() -> Self {
        Self {
            components: ArrayVec::new(),
            transitions: ArrayVec::new(),
            entry_shim_component_id: None,
            exit_shim_component_id: None,
            log: ArrayVec::new(),
        }
    }

    // Registers a component descriptor before any code bytes are staged.
    pub fn create_component(&mut self, descriptor: ComponentDescriptor) -> Result<()> {
        if self
            .components
            .iter()
            .any(|c| c.descriptor.component_id == descriptor.component_id)
        {
            return Err(LiberumError::DuplicateComponentId);
        }
        self.push_log(DemoLogEvent::ComponentCreated(
            descriptor.component_id,
            descriptor.component_type,
        ))?;
        self.components
            .try_push(ManagedComponent {
                descriptor,
                state: ComponentState::Created,
                bytes_loaded: 0,
            })
            .map_err(|_| LiberumError::CapacityExceeded)
    }

    // Marks a component as loaded once its backing image has been staged.
    pub fn load_component(&mut self, component_id: u64, bytes_loaded: usize) -> Result<()> {
        let component = self.component_mut(component_id)?;
        component.state = ComponentState::Loaded;
        component.bytes_loaded = bytes_loaded;
        self.push_log(DemoLogEvent::ComponentLoaded(component_id))
    }

    // Freezes a component descriptor and pins the measurement used for later
    // entry verification.
    pub fn finalize_component(
        &mut self,
        component_id: u64,
        finalized_measurement: [u8; SHA384_DIGEST_BYTES],
    ) -> Result<()> {
        let component = self.component_mut(component_id)?;
        if component.state != ComponentState::Loaded {
            return Err(LiberumError::ComponentNotLoaded);
        }
        component.descriptor.measurement = finalized_measurement;
        component.descriptor.finalized = true;
        component.state = ComponentState::Finalized;
        self.push_log(DemoLogEvent::ComponentFinalized(component_id))
    }

    pub fn register_entry_shim(&mut self, component_id: u64) -> Result<()> {
        let component = self.component(component_id)?;
        if component.descriptor.component_type != ComponentType::EntryShim {
            return Err(LiberumError::InvalidState);
        }
        self.entry_shim_component_id = Some(component_id);
        self.push_log(DemoLogEvent::EntryShimRegistered(component_id))
    }

    pub fn register_exit_shim(&mut self, component_id: u64) -> Result<()> {
        let component = self.component(component_id)?;
        if component.descriptor.component_type != ComponentType::ExitShim {
            return Err(LiberumError::InvalidState);
        }
        self.exit_shim_component_id = Some(component_id);
        self.push_log(DemoLogEvent::ExitShimRegistered(component_id))
    }

    // Adds one allowed edge to the component transition graph.
    pub fn authorize_transition(
        &mut self,
        from_component_id: u64,
        to_component_id: u64,
        target_pc: u64,
    ) -> Result<()> {
        let from_component = self.component(from_component_id)?;
        let to_component = self.component(to_component_id)?;
        if !from_component.descriptor.finalized || !to_component.descriptor.finalized {
            return Err(LiberumError::ComponentNotFinalized);
        }
        self.transitions
            .try_push(AuthorizedTransition {
                from_component_id,
                to_component_id,
                target_pc,
            })
            .map_err(|_| LiberumError::CapacityExceeded)
    }

    // Checks both graph authorization and descriptor-local entry policy.
    pub fn verify_jmp(
        &mut self,
        from_component_id: u64,
        to_component_id: u64,
        target_pc: u64,
        measurement: [u8; SHA384_DIGEST_BYTES],
    ) -> Result<()> {
        if !self.is_transition_authorized(from_component_id, to_component_id, target_pc) {
            self.push_log(DemoLogEvent::VerifyJmpRejected(
                to_component_id,
                target_pc,
                LiberumError::TransitionNotAuthorized,
            ))?;
            return Err(LiberumError::TransitionNotAuthorized);
        }
        self.verify_entry(to_component_id, target_pc, measurement)
    }

    // Enforces the minimum verify_jmp semantics for the current PoC:
    // finalized component, exact entry PC, in-range target, and matching digest.
    pub fn verify_entry(
        &mut self,
        to_component_id: u64,
        target_pc: u64,
        measurement: [u8; SHA384_DIGEST_BYTES],
    ) -> Result<()> {
        let component = self.component(to_component_id)?;
        let descriptor = component.descriptor.clone();
        let result = if !descriptor.finalized {
            Err(LiberumError::ComponentNotFinalized)
        } else if descriptor.entry_pc != target_pc {
            Err(LiberumError::EntryPointMismatch)
        } else if !descriptor.contains(target_pc) {
            Err(LiberumError::TargetOutsideCodeRegion)
        } else if descriptor.measurement != measurement {
            Err(LiberumError::MeasurementMismatch)
        } else {
            Ok(())
        };

        match result {
            Ok(()) => self.push_log(DemoLogEvent::VerifyJmpAccepted(to_component_id, target_pc)),
            Err(err) => {
                self.push_log(DemoLogEvent::VerifyJmpRejected(to_component_id, target_pc, err))?;
                Err(err)
            }
        }
    }

    // Starts a confidential execution window once verify_jmp has succeeded.
    pub fn begin_confidential_run(
        &mut self,
        component_id: u64,
    ) -> Result<ConfidentialExecutionContext> {
        if self.entry_shim_component_id.is_none() {
            return Err(LiberumError::EntryShimMissing);
        }
        let component = self.component_mut(component_id)?;
        if component.state != ComponentState::Finalized {
            return Err(LiberumError::ComponentNotFinalized);
        }
        component.state = ComponentState::Running;
        self.push_log(DemoLogEvent::EnteredConfidential(component_id))?;
        Ok(ConfidentialExecutionContext {
            active_component_id: component_id,
            sandbox_active: true,
            trusted_exit_pending: false,
            last_exit_reason: None,
        })
    }

    // Converts an in-flight event into a trusted exit through the registered
    // exit shim rather than returning directly to the host path.
    pub fn force_trusted_exit(
        &mut self,
        ctx: &mut ConfidentialExecutionContext,
        reason: ForcedExitReason,
    ) -> Result<u64> {
        let exit_shim_id = self
            .exit_shim_component_id
            .ok_or(LiberumError::ExitShimMissing)?;
        let exit_shim = self.component(exit_shim_id)?;
        let exit_pc = exit_shim
            .descriptor
            .exit_pc
            .or(Some(exit_shim.descriptor.entry_pc))
            .ok_or(LiberumError::ExitShimMissing)?;

        ctx.trusted_exit_pending = true;
        ctx.last_exit_reason = Some(reason);
        ctx.sandbox_active = false;

        if let Ok(component) = self.component_mut(ctx.active_component_id) {
            component.state = ComponentState::Exited;
        }

        self.push_log(DemoLogEvent::ForcedTrustedExit(
            ctx.active_component_id,
            reason,
            exit_pc,
        ))?;
        Ok(exit_pc)
    }

    pub fn log(&self) -> &[DemoLogEvent] {
        self.log.as_slice()
    }

    pub fn descriptor(&self, component_id: u64) -> Result<&ComponentDescriptor> {
        Ok(&self.component(component_id)?.descriptor)
    }

    pub fn measurement(&self, component_id: u64) -> Result<[u8; SHA384_DIGEST_BYTES]> {
        Ok(self.component(component_id)?.descriptor.measurement)
    }

    fn push_log(&mut self, event: DemoLogEvent) -> Result<()> {
        self.log
            .try_push(event)
            .map_err(|_| LiberumError::CapacityExceeded)
    }

    fn is_transition_authorized(
        &self,
        from_component_id: u64,
        to_component_id: u64,
        target_pc: u64,
    ) -> bool {
        self.transitions.iter().any(|t| {
            t.from_component_id == from_component_id
                && t.to_component_id == to_component_id
                && t.target_pc == target_pc
        })
    }

    fn component(&self, component_id: u64) -> Result<&ManagedComponent> {
        self.components
            .iter()
            .find(|c| c.descriptor.component_id == component_id)
            .ok_or(LiberumError::ComponentNotFound)
    }

    fn component_mut(&mut self, component_id: u64) -> Result<&mut ManagedComponent> {
        self.components
            .iter_mut()
            .find(|c| c.descriptor.component_id == component_id)
            .ok_or(LiberumError::ComponentNotFound)
    }
}

