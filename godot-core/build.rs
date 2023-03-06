/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

fn main() {
    let gen_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gen/"));

    if gen_path.exists() {
        std::fs::remove_dir_all(gen_path).unwrap_or_else(|e| panic!("failed to delete dir: {e}"));
    }

    let build_config = format!("float_{}", std::env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap());
    println!("Selected build configuration: {build_config}");
    godot_codegen::generate_core_files(gen_path, build_config.as_str());
}
