// SPDX-License-Identifier: (GPL-2.0-or-later OR MIT)
//
// Copyright (c) 2022 SUSE LLC
//
// Author: Joerg Roedel <jroedel@suse.de>
//
// vim: ts=4 sw=4 et

use crate::types::{VirtAddr, PAGE_SIZE, PAGE_SIZE_2M};
use crate::utils::is_aligned;
use core::arch::asm;

const PV_ERR_FAIL_SIZE_MISMATCH: u64 = 6;

#[derive(Debug)]
pub struct PValidateError {
    pub error_code: u64,
    pub changed: bool,
}

impl PValidateError {
    pub fn new(code: u64, changed: bool) -> Self {
        PValidateError { error_code: code, changed: changed }
    }
}

fn pvalidate_range_4k(start: VirtAddr, end: VirtAddr, valid: bool) -> Result<(), PValidateError> {
    for addr in (start..end).step_by(PAGE_SIZE) {
        pvalidate(addr, false, valid)?;
    }

    Ok(())
}

pub fn pvalidate_range(start: VirtAddr, end: VirtAddr, valid: bool) -> Result<(), PValidateError> {
    let mut addr = start;

    while addr < end {
        if is_aligned(addr, PAGE_SIZE_2M) && (addr + PAGE_SIZE_2M) <= end {
            if let Err(e) = pvalidate(addr, true, valid) {
                if e.error_code == PV_ERR_FAIL_SIZE_MISMATCH {
                    pvalidate_range_4k(addr, addr + PAGE_SIZE_2M, valid)?;
                } else {
                    return Err(e);
                }
            }
            addr += PAGE_SIZE_2M;
        } else {
            pvalidate(addr, false, valid)?;
            addr += PAGE_SIZE;
        }
    }

    Ok(())
}

pub fn pvalidate(vaddr: VirtAddr, huge_page: bool, valid: bool) -> Result<(), PValidateError> {
    let rax = vaddr;
    let rcx = huge_page as u64;
    let rdx = valid as u64;
    let ret: u64;
    let cf: u64;

    unsafe {
        asm!(".byte 0xf2, 0x0f, 0x01, 0xff",
             "xorq %rcx, %rcx",
             "jnc 1f",
             "incq %rcx",
             "1:",
             in("rax")  rax,
             in("rcx")  rcx,
             in("rdx")  rdx,
             lateout("rax") ret,
             lateout("rcx") cf,
             options(att_syntax));
    }

    let changed : bool = cf == 0;

    if ret == 0 && changed {
        Ok(())
    } else {
        Err(PValidateError::new(ret, changed))
    }
}

pub fn raw_vmgexit() {
    unsafe {
        asm!("rep; vmmcall", options(att_syntax));
    }
}

bitflags::bitflags! {
    pub struct RMPFlags: u64 {
        const VMPL0 = 0;
        const VMPL1 = 1;
        const VMPL2 = 2;
        const VMPL3 = 3;
        const READ = 1u64 << 8;
        const WRITE = 1u64 << 9;
        const X_USER = 1u64 << 10;
        const X_SUPER = 1u64 << 11;
        const BIT_VMSA = 1u64 << 16;
        const NONE = 0;
        const RWX = Self::READ.bits | Self::WRITE.bits | Self::X_USER.bits | Self::X_SUPER.bits;
        const VMSA = Self::READ.bits | Self::BIT_VMSA.bits;
    }
}

pub fn rmp_adjust(addr: VirtAddr, flags: RMPFlags, huge: bool) -> Result<(), u64> {
    let rcx: usize = if huge { 1 } else { 0 };
    let rax: u64 = addr as u64;
    let rdx: u64 = flags.bits();
    let mut result: u64;
    let mut ex: u64;

    unsafe {
        asm!("1: .byte 0xf3, 0x0f, 0x01, 0xfe
                 xorq %rcx, %rcx
              2:
              .pushsection \"__exception_table\",\"a\"
              .balign 16
              .quad (1b)
              .quad (2b)
              .popsection",
                in("rax") rax,
                in("rcx") rcx,
                in("rdx") rdx,
                lateout("rax") result,
                lateout("rcx") ex,
                options(att_syntax));
    }

    if result == 0 && ex == 0 {
        // RMPADJUST completed successfully
        Ok(())
    } else if ex == 0 {
        // RMPADJUST instruction completed with failure
        Err(result)
    } else {
        // Report exceptions on RMPADJUST just as FailInput
        Err(1u64)
    }
}

fn rmpadjust_adjusted_error(vaddr: VirtAddr, flags: RMPFlags, huge: bool) -> Result<(), u64> {
    rmp_adjust(vaddr, flags, huge)
        .map_err(|code| if code < 0x10 { code } else { 0x11 })
}

pub fn rmp_revoke_guest_access(vaddr: VirtAddr, huge: bool) -> Result<(), u64> {
    rmpadjust_adjusted_error(vaddr, RMPFlags::VMPL1 | RMPFlags::NONE, huge)?;
    rmpadjust_adjusted_error(vaddr, RMPFlags::VMPL2 | RMPFlags::NONE, huge)?;
    rmpadjust_adjusted_error(vaddr, RMPFlags::VMPL3 | RMPFlags::NONE, huge)
}

pub fn rmp_grant_guest_access(vaddr: VirtAddr, huge: bool) -> Result<(), u64> {
    rmpadjust_adjusted_error(vaddr, RMPFlags::VMPL1 | RMPFlags::RWX, huge)
}

pub fn rmp_set_guest_vmsa(vaddr: VirtAddr) -> Result<(), u64> {
    rmp_revoke_guest_access(vaddr, false)?;
    rmpadjust_adjusted_error(vaddr, RMPFlags::VMPL1 | RMPFlags::VMSA, false)
}

pub fn rmp_clear_guest_vmsa(vaddr: VirtAddr) -> Result<(), u64> {
    rmp_revoke_guest_access(vaddr, false)?;
    rmp_grant_guest_access(vaddr, false)
}

