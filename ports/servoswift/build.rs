/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let output_file = target_dir()
        .join("include")
        .join("servoswift.h");

    // Only generate bindings if cbindgen is available and we can parse the code
    if let Ok(config) = cbindgen::Config::from_file("cbindgen.toml") {
        match cbindgen::Builder::new()
            .with_crate(crate_dir)
            .with_config(config)
            .generate()
        {
            Ok(bindings) => {
                bindings.write_to_file(output_file);
            }
            Err(e) => {
                println!("cargo:warning=Could not generate bindings: {:?}", e);
                // Create a minimal header file as fallback
                std::fs::create_dir_all(output_file.parent().unwrap()).unwrap();
                std::fs::write(output_file, "// ServoSwift C bindings will be generated here\n").unwrap();
            }
        }
    }
}

fn target_dir() -> PathBuf {
    if let Ok(target) = env::var("CARGO_TARGET_DIR") {
        PathBuf::from(target)
    } else {
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("target")
    }
}
