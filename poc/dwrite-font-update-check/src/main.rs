//! Times IDWriteFactory::GetSystemFontCollection with checkForUpdates = true and false.
//!
//! GPUI's Windows text system calls it with `true` when it starts
//! (crates/gpui_windows/src/direct_write.rs, `DirectWriteTextSystem::new`).
//! Run once per process: `dwrite-font-update-check true` or `... false`.

use std::time::Instant;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWriteCreateFactory, IDWriteFactory5,
};

fn main() -> windows::core::Result<()> {
    let check_for_updates = match std::env::args().nth(1).as_deref() {
        Some("true") => true,
        Some("false") => false,
        _ => {
            eprintln!("usage: dwrite-font-update-check true|false");
            std::process::exit(2);
        }
    };

    let start = Instant::now();
    let factory: IDWriteFactory5 = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };
    let factory_ms = start.elapsed().as_secs_f64() * 1000.;

    let start = Instant::now();
    let mut collection = None;
    unsafe { factory.GetSystemFontCollection(false, &mut collection, check_for_updates)? };
    let collection_ms = start.elapsed().as_secs_f64() * 1000.;
    let families = collection.map_or(0, |c| unsafe { c.GetFontFamilyCount() });

    println!(
        "checkForUpdates={check_for_updates} GetSystemFontCollection={collection_ms:.1} ms \
         (DWriteCreateFactory={factory_ms:.1} ms, families={families})"
    );
    Ok(())
}
