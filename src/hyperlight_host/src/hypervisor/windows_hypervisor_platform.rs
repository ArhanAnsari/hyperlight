use core::ffi::c_void;

use tracing::{instrument, Span};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Hypervisor::*;

use super::wrappers::HandleWrapper;
use crate::hypervisor::wrappers::{WHvFPURegisters, WHvGeneralRegisters, WHvSpecialRegisters};
use crate::mem::memory_region::{MemoryRegion, MemoryRegionFlags};
use crate::Result;

// We need to pass in a primitive array of register names/values
// to WHvSetVirtualProcessorRegisters and rust needs to know array size
// at compile time. There is an assert in set_virtual_process_registers
// to ensure we never try and set more registers than this constant
const REGISTER_COUNT: usize = 16;

/// Interop calls for Windows Hypervisor Platform APIs
///
/// Documentation can be found at:
/// - https://learn.microsoft.com/en-us/virtualization/api/hypervisor-platform/hypervisor-platform
/// - https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Hypervisor/index.html
#[instrument(skip_all, parent = Span::current(), level= "Trace")]
pub(crate) fn is_hypervisor_present() -> bool {
    let mut capability: WHV_CAPABILITY = Default::default();
    let written_size: Option<*mut u32> = None;

    match unsafe {
        WHvGetCapability(
            WHvCapabilityCodeHypervisorPresent,
            &mut capability as *mut _ as *mut c_void,
            std::mem::size_of::<WHV_CAPABILITY>() as u32,
            written_size,
        )
    } {
        Ok(_) => unsafe { capability.HypervisorPresent.as_bool() },
        Err(_) => false,
    }
}

#[derive(Debug)]
pub(super) struct VMPartition(WHV_PARTITION_HANDLE);

impl VMPartition {
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn new(proc_count: u32) -> Result<Self> {
        let hdl = unsafe { WHvCreatePartition() }?;
        Self::set_processor_count(&hdl, proc_count)?;
        unsafe { WHvSetupPartition(hdl) }?;
        Ok(Self(hdl))
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    fn set_processor_count(
        partition_handle: &WHV_PARTITION_HANDLE,
        processor_count: u32,
    ) -> Result<()> {
        unsafe {
            WHvSetPartitionProperty(
                *partition_handle,
                WHvPartitionPropertyCodeProcessorCount,
                &processor_count as *const u32 as *const c_void,
                std::mem::size_of_val(&processor_count) as u32,
            )?;
        }

        Ok(())
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn map_gpa_range(
        &mut self,
        regions: &[MemoryRegion],
        process_handle: HandleWrapper,
    ) -> Result<()> {
        let process_handle: HANDLE = process_handle.into();
        regions.iter().try_for_each(|region| unsafe {
            WHvMapGpaRange2(
                self.0,
                process_handle,
                region.host_region.start as *const c_void,
                region.guest_region.start as u64,
                (region.guest_region.end - region.guest_region.start) as u64,
                region
                    .flags
                    .iter()
                    .filter_map(|flag| match flag {
                        MemoryRegionFlags::NONE => Some(WHvMapGpaRangeFlagNone),
                        MemoryRegionFlags::READ => Some(WHvMapGpaRangeFlagRead),
                        MemoryRegionFlags::WRITE => Some(WHvMapGpaRangeFlagWrite),
                        MemoryRegionFlags::EXECUTE => Some(WHvMapGpaRangeFlagExecute),
                        MemoryRegionFlags::STACK_GUARD => None,
                        _ => panic!("Invalid flag"),
                    })
                    .fold(WHvMapGpaRangeFlagNone, |acc, flag| acc | flag), // collect using bitwise OR,
            )
        })?;
        Ok(())
    }
}

impl Drop for VMPartition {
    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    fn drop(&mut self) {
        if let Err(e) = unsafe { WHvDeletePartition(self.0) } {
            tracing::error!(
                "Failed to delete partition (WHvDeletePartition failed): {:?}",
                e
            );
        }
    }
}

#[derive(Debug)]
pub(super) struct VMProcessor(VMPartition);
impl VMProcessor {
    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn new(part: VMPartition) -> Result<Self> {
        unsafe { WHvCreateVirtualProcessor(part.0, 0, 0) }?;
        Ok(Self(part))
    }

    #[instrument(skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_partition_hdl(&self) -> WHV_PARTITION_HANDLE {
        let part = &self.0;
        part.0
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn set_registers(
        &mut self,
        registers: &[(WHV_REGISTER_NAME, WHV_REGISTER_VALUE)],
    ) -> Result<()> {
        let partition_handle = self.get_partition_hdl();
        let register_count = registers.len();
        assert!(register_count <= REGISTER_COUNT);
        let mut register_names: Vec<WHV_REGISTER_NAME> = vec![];
        let mut register_values: Vec<WHV_REGISTER_VALUE> = vec![];

        for (key, value) in registers.iter() {
            register_names.push(*key);
            register_values.push(*value);
        }

        unsafe {
            WHvSetVirtualProcessorRegisters(
                partition_handle,
                0,
                register_names.as_ptr(),
                register_count as u32,
                register_values.as_ptr(),
            )?;
        }

        Ok(())
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn get_sregs(&self) -> Result<WHvSpecialRegisters> {
        const LEN: usize = 17;

        let names: [WHV_REGISTER_NAME; LEN] = [
            WHvX64RegisterCr0,
            WHvX64RegisterCr2,
            WHvX64RegisterCr3,
            WHvX64RegisterCr4,
            WHvX64RegisterCr8,
            WHvX64RegisterEfer,
            WHvX64RegisterApicBase,
            WHvX64RegisterCs,
            WHvX64RegisterDs,
            WHvX64RegisterEs,
            WHvX64RegisterFs,
            WHvX64RegisterGs,
            WHvX64RegisterSs,
            WHvX64RegisterTr,
            WHvX64RegisterLdtr,
            WHvX64RegisterGdtr,
            WHvX64RegisterIdtr,
        ];

        let mut out: [WHV_REGISTER_VALUE; LEN] = unsafe { std::mem::zeroed() };
        unsafe {
            WHvGetVirtualProcessorRegisters(
                self.get_partition_hdl(),
                0,
                names.as_ptr(),
                LEN as u32,
                out.as_mut_ptr(),
            )?;
        }

        let res: WHvSpecialRegisters = WHvSpecialRegisters {
            cr0: out[0],
            cr2: out[1],
            cr3: out[2],
            cr4: out[3],
            cr8: out[4],
            efer: out[5],
            apic_base: out[6],
            cs: out[7],
            ds: out[8],
            es: out[9],
            fs: out[10],
            gs: out[11],
            ss: out[12],
            tr: out[13],
            ldtr: out[14],
            gdtr: out[15],
            idtr: out[16],
        };

        Ok(res)
    }

    // Sets the registers for the VMProcessor to the given general purpose registers.
    // If you want to set other registers, use `set_registers` instead.
    pub(super) fn set_general_purpose_registers(
        &mut self,
        regs: &WHvGeneralRegisters,
    ) -> Result<()> {
        const LEN: usize = 18;

        let names: [WHV_REGISTER_NAME; LEN] = [
            WHvX64RegisterRax,
            WHvX64RegisterRbx,
            WHvX64RegisterRcx,
            WHvX64RegisterRdx,
            WHvX64RegisterRsi,
            WHvX64RegisterRdi,
            WHvX64RegisterRsp,
            WHvX64RegisterRbp,
            WHvX64RegisterR8,
            WHvX64RegisterR9,
            WHvX64RegisterR10,
            WHvX64RegisterR11,
            WHvX64RegisterR12,
            WHvX64RegisterR13,
            WHvX64RegisterR14,
            WHvX64RegisterR15,
            WHvX64RegisterRip,
            WHvX64RegisterRflags,
        ];

        let values: [WHV_REGISTER_VALUE; LEN] = [
            WHV_REGISTER_VALUE { Reg64: regs.rax },
            WHV_REGISTER_VALUE { Reg64: regs.rbx },
            WHV_REGISTER_VALUE { Reg64: regs.rcx },
            WHV_REGISTER_VALUE { Reg64: regs.rdx },
            WHV_REGISTER_VALUE { Reg64: regs.rsi },
            WHV_REGISTER_VALUE { Reg64: regs.rdi },
            WHV_REGISTER_VALUE { Reg64: regs.rsp },
            WHV_REGISTER_VALUE { Reg64: regs.rbp },
            WHV_REGISTER_VALUE { Reg64: regs.r8 },
            WHV_REGISTER_VALUE { Reg64: regs.r9 },
            WHV_REGISTER_VALUE { Reg64: regs.r10 },
            WHV_REGISTER_VALUE { Reg64: regs.r11 },
            WHV_REGISTER_VALUE { Reg64: regs.r12 },
            WHV_REGISTER_VALUE { Reg64: regs.r13 },
            WHV_REGISTER_VALUE { Reg64: regs.r14 },
            WHV_REGISTER_VALUE { Reg64: regs.r15 },
            WHV_REGISTER_VALUE { Reg64: regs.rip },
            WHV_REGISTER_VALUE { Reg64: regs.rflags },
        ];

        unsafe {
            WHvSetVirtualProcessorRegisters(
                self.get_partition_hdl(),
                0,
                names.as_ptr(),
                LEN as u32,
                values.as_ptr(),
            )?;
        }
        Ok(())
    }

    pub(super) fn get_regs(&self) -> Result<WHvGeneralRegisters> {
        const LEN: usize = 18;

        let names: [WHV_REGISTER_NAME; LEN] = [
            WHvX64RegisterRax,
            WHvX64RegisterRbx,
            WHvX64RegisterRcx,
            WHvX64RegisterRdx,
            WHvX64RegisterRsi,
            WHvX64RegisterRdi,
            WHvX64RegisterRsp,
            WHvX64RegisterRbp,
            WHvX64RegisterR8,
            WHvX64RegisterR9,
            WHvX64RegisterR10,
            WHvX64RegisterR11,
            WHvX64RegisterR12,
            WHvX64RegisterR13,
            WHvX64RegisterR14,
            WHvX64RegisterR15,
            WHvX64RegisterRip,
            WHvX64RegisterRflags,
        ];

        let mut out: [WHV_REGISTER_VALUE; LEN] = unsafe { std::mem::zeroed() };
        unsafe {
            WHvGetVirtualProcessorRegisters(
                self.get_partition_hdl(),
                0,
                names.as_ptr(),
                LEN as u32,
                out.as_mut_ptr(),
            )?;
            Ok(WHvGeneralRegisters {
                rax: out[0].Reg64,
                rbx: out[1].Reg64,
                rcx: out[2].Reg64,
                rdx: out[3].Reg64,
                rsi: out[4].Reg64,
                rdi: out[5].Reg64,
                rsp: out[6].Reg64,
                rbp: out[7].Reg64,
                r8: out[8].Reg64,
                r9: out[9].Reg64,
                r10: out[10].Reg64,
                r11: out[11].Reg64,
                r12: out[12].Reg64,
                r13: out[13].Reg64,
                r14: out[14].Reg64,
                r15: out[15].Reg64,
                rip: out[16].Reg64,
                rflags: out[17].Reg64,
            })
        }
    }

    pub(super) fn set_fpu(&mut self, regs: &WHvFPURegisters) -> Result<()> {
        const LEN: usize = 26;

        let names: [WHV_REGISTER_NAME; LEN] = [
            WHvX64RegisterXmm0,
            WHvX64RegisterXmm1,
            WHvX64RegisterXmm2,
            WHvX64RegisterXmm3,
            WHvX64RegisterXmm4,
            WHvX64RegisterXmm5,
            WHvX64RegisterXmm6,
            WHvX64RegisterXmm7,
            WHvX64RegisterXmm8,
            WHvX64RegisterXmm9,
            WHvX64RegisterXmm10,
            WHvX64RegisterXmm11,
            WHvX64RegisterXmm12,
            WHvX64RegisterXmm13,
            WHvX64RegisterXmm14,
            WHvX64RegisterXmm15,
            WHvX64RegisterFpMmx0,
            WHvX64RegisterFpMmx1,
            WHvX64RegisterFpMmx2,
            WHvX64RegisterFpMmx3,
            WHvX64RegisterFpMmx4,
            WHvX64RegisterFpMmx5,
            WHvX64RegisterFpMmx6,
            WHvX64RegisterFpMmx7,
            WHvX64RegisterFpControlStatus,
            WHvX64RegisterXmmControlStatus,
        ];

        let xmm_regs = [
            regs.xmm0, regs.xmm1, regs.xmm2, regs.xmm3, regs.xmm4, regs.xmm5, regs.xmm6, regs.xmm7,
            regs.xmm8, regs.xmm9, regs.xmm10, regs.xmm11, regs.xmm12, regs.xmm13, regs.xmm14,
            regs.xmm15,
        ];

        let mut values: Vec<WHV_REGISTER_VALUE> = xmm_regs
            .iter()
            .map(|&reg| WHV_REGISTER_VALUE {
                Fp: WHV_X64_FP_REGISTER {
                    AsUINT128: WHV_UINT128 {
                        Anonymous: WHV_UINT128_0 {
                            Low64: reg as u64,
                            High64: (reg >> 64) as u64,
                        },
                    },
                },
            })
            .collect();

        values.extend_from_slice(&[
            WHV_REGISTER_VALUE { Reg64: regs.mmx0 },
            WHV_REGISTER_VALUE { Reg64: regs.mmx1 },
            WHV_REGISTER_VALUE { Reg64: regs.mmx2 },
            WHV_REGISTER_VALUE { Reg64: regs.mmx3 },
            WHV_REGISTER_VALUE { Reg64: regs.mmx4 },
            WHV_REGISTER_VALUE { Reg64: regs.mmx5 },
            WHV_REGISTER_VALUE { Reg64: regs.mmx6 },
            WHV_REGISTER_VALUE { Reg64: regs.mmx7 },
            WHV_REGISTER_VALUE {
                FpControlStatus: WHV_X64_FP_CONTROL_STATUS_REGISTER {
                    Anonymous: WHV_X64_FP_CONTROL_STATUS_REGISTER_0 {
                        FpControl: regs.fp_control_word,
                        FpTag: regs.fp_tag_word,
                        ..Default::default()
                    },
                },
            },
            WHV_REGISTER_VALUE {
                XmmControlStatus: WHV_X64_XMM_CONTROL_STATUS_REGISTER {
                    Anonymous: WHV_X64_XMM_CONTROL_STATUS_REGISTER_0 {
                        XmmStatusControl: regs.mxcsr,
                        ..Default::default()
                    },
                },
            },
        ]);

        unsafe {
            WHvSetVirtualProcessorRegisters(
                self.get_partition_hdl(),
                0,
                names.as_ptr(),
                LEN as u32,
                values.as_ptr(),
            )?;
        }
        Ok(())
    }

    #[instrument(err(Debug), skip_all, parent = Span::current(), level= "Trace")]
    pub(super) fn run(&mut self) -> Result<WHV_RUN_VP_EXIT_CONTEXT> {
        let partition_handle = self.get_partition_hdl();
        let mut exit_context: WHV_RUN_VP_EXIT_CONTEXT = Default::default();

        unsafe {
            WHvRunVirtualProcessor(
                partition_handle,
                0,
                &mut exit_context as *mut _ as *mut c_void,
                std::mem::size_of::<WHV_RUN_VP_EXIT_CONTEXT>() as u32,
            )?;
        }

        Ok(exit_context)
    }
}

impl Drop for VMProcessor {
    #[instrument(parent = Span::current(), level= "Trace")]
    fn drop(&mut self) {
        let part_hdl = self.get_partition_hdl();
        if let Err(e) = unsafe { WHvDeleteVirtualProcessor(part_hdl, 0) } {
            tracing::error!(
                "Failed to delete virtual processor (WHvDeleteVirtualProcessor failed): {:?}",
                e
            );
        }
    }
}
