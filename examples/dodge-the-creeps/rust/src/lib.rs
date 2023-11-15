/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot::prelude::*;

mod hud;
mod main_scene;
mod mob;
mod player;

struct DodgeTheCreeps;

#[gdextension]
unsafe impl ExtensionLibrary for DodgeTheCreeps {}

use std::ffi::CString;
use core::ffi::c_char;
#[cfg(target_os = "emscripten")]
extern "C" {
    fn emscripten_debugger();
    fn emscripten_run_script(script: *const c_char);
}
fn debugger() {
    #[cfg(target_os = "emscripten")]
    unsafe { emscripten_debugger(); }
}
#[cfg(target_os = "emscripten")]
fn run_script(script: &str) {
    let cs = CString::new(script).expect("Unable to create CString from script");
    unsafe {
        emscripten_run_script(cs.as_ptr());
    }
}
