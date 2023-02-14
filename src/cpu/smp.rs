// SPDX-License-Identifier: (GPL-2.0-or-later OR MIT)
//
// Copyright (c) 2022 SUSE LLC
//
// Author: Joerg Roedel <jroedel@suse.de>
//
// vim: ts=4 sw=4 et

extern crate alloc;

use crate::cpu::percpu::{PerCpu, this_cpu_mut};
use crate::cpu::vmsa::init_svsm_vmsa;
use crate::mm::virt_to_phys;
use crate::types::VirtAddr;
use crate::sev::vmsa::VMSA;
use crate::acpi::tables::ACPICPUInfo;
use alloc::vec::Vec;

fn start_cpu(apic_id: u32) {
    unsafe {
        let start_rip: u64 = (start_ap as *const u8) as u64;
        let percpu = PerCpu::alloc()
            .expect("Failed to allocate AP per-cpu data")
            .as_mut()
            .unwrap();

        percpu.setup().expect("Failed to setup AP per-cpu area");
        percpu.set_apic_id(apic_id);
        percpu.alloc_vmsa(0).expect("Failed to allocate AP SVSM VMSA");


        init_svsm_vmsa(percpu.vmsa(0));
        percpu.prepare_svsm_vmsa(start_rip);

        let vmsa_addr = (percpu.vmsa(0) as *const VMSA) as VirtAddr;
        let vmsa_pa = virt_to_phys(vmsa_addr);
        let sev_features = percpu.vmsa(0).sev_features;

        percpu.vmsa(0).enable();
        this_cpu_mut().ghcb().ap_create(vmsa_pa, apic_id.into(), 0, sev_features)
            .expect("Failed to launch secondary CPU");
       loop {
          if percpu.is_online() {
             break;
          }
       } 
    }
}

pub fn start_secondary_cpus(cpus: &Vec<ACPICPUInfo>) {
    let mut count: usize = 0;
    for c in cpus.iter().filter(|c| c.apic_id != 0 && c.enabled) {
        log::info!("Launching AP with APIC-ID {}", c.apic_id);
        start_cpu(c.apic_id);
        count += 1;
    }
    log::info!("Brough {} AP(s) online", count);
}

#[no_mangle]
fn start_ap() {
    this_cpu_mut().setup_on_cpu().expect("setup_on_cpu() failed");

    // Send a life-sign
    log::info!("AP with APIC-ID {} is online", this_cpu_mut().get_apic_id());

    // Set CPU online so that BSP can proceed
    this_cpu_mut().set_online();

    // Loop for now
    loop {}
}
