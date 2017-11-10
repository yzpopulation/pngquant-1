/*
** © 2017 by Kornel Lesiński.
**
** See COPYRIGHT file for license.
*/

#![allow(unused_extern_crates)]
#![cfg_attr(feature="alloc_system", feature(alloc_system))]

#[cfg(feature="alloc_system")]
extern crate alloc_system;

#[cfg(feature = "openmp")]
extern crate openmp_sys;

extern crate imagequant_sys;
#[cfg(feature = "cocoa")]
extern crate cocoa_image;
extern crate libpng_sys;
extern crate libc;
extern crate getopts;
extern crate wild;

#[cfg(feature = "cocoa")]
pub mod rwpng_cocoa;

#[cfg(feature = "lcms2")]
extern crate lcms2_sys;

use std::os::raw::{c_uint, c_char};
use std::process;
use std::ptr;
use std::io;
use std::io::prelude::*;
use std::ffi::CString;

mod ffi;
use ffi::*;

const VERSION_SUFFIX: &'static str = "(November 2017; Rust)";

fn unwrap_ptr(opt: Option<&CString>) -> *const c_char {
    opt.map(|c| c.as_ptr()).unwrap_or(ptr::null())
}

fn print_full_version(out: &mut Write) -> io::Result<()> {
    write!(out, "pngquant, {} {}, by Kornel Lesinski, Greg Roelofs.\n", env!("CARGO_PKG_VERSION"), VERSION_SUFFIX)?;

    #[cfg(debug_assertions)]
    out.write_all(b"   WARNING: this is a DEBUG (slow) version.\n")?;

    #[cfg(not(feature = "sse"))]
    out.write_all(b"   SSE acceleration disabled.\n")?;

    #[cfg(feature = "openmp")]
    out.write_all(b"   Compiled with OpenMP (multicore support).\n")?;
    // rwpng_version_info(fd);
    out.write_all(b"\n")
}

fn print_usage(out: &mut Write) -> io::Result<()> {
    out.write_all(b"\
usage:  pngquant [options] [ncolors] -- pngfile [pngfile ...]\n\
        pngquant [options] [ncolors] - >stdout <stdin\n\n\
options:\n\
  --force           overwrite existing output files (synonym: -f)\n\
  --skip-if-larger  only save converted files if they're smaller than original\n\
  --output file     destination file path to use instead of --ext (synonym: -o)\n\
  --ext new.png     set custom suffix/extension for output filenames\n\
  --quality min-max don't save below min, use fewer colors below max (0-100)\n\
  --speed N         speed/quality trade-off. 1=slow, 3=default, 11=fast & rough\n\
  --nofs            disable Floyd-Steinberg dithering\n\
  --posterize N     output lower-precision color (e.g. for ARGB4444 output)\n\
  --strip           remove optional metadata (default on Mac)\n\
  --verbose         print status messages (synonym: -v)\n\
\n\
Quantizes one or more 32-bit RGBA PNGs to 8-bit (or smaller) RGBA-palette.\n\
The output filename is the same as the input name except that\n\
it ends in \"-fs8.png\", \"-or8.png\" or your custom extension (unless the\n\
input is stdin, in which case the quantized image will go to stdout).\n\
If you pass the special output path \"-\" and a single input file, that file\n\
will be processed and the quantized image will go to stdout.\n\
The default behavior if the output file exists is to skip the conversion;\n\
use --force to overwrite. See man page for full list of options.\n")
}

fn main() {
    let mut opts = getopts::Options::new();

    opts.optflag("v", "verbose", "");
    opts.optflag("h", "help", "");
    opts.optflag("q", "quiet", "");
    opts.optflag("f", "force", "");
    opts.optflag("", "no-force", "");
    opts.optflag("", "ordered", "");
    opts.optflag("", "nofs", "");
    opts.optflag("", "iebug", "");
    opts.optflag("", "transbug", "");
    opts.optflag("", "skip-if-larger", "");
    opts.optflag("", "strip", "");
    opts.optflag("V", "version", "");
    opts.optflagopt("", "floyd", "0.0-1.0", "");
    opts.optopt("", "ext", "extension", "");
    opts.optopt("o", "output", "file", "");
    opts.optopt("s", "speed", "3", "");
    opts.optopt("Q", "quality", "0-100", "");
    opts.optopt("", "posterize", "0", "");
    opts.optopt("", "map", "png", "");

    let args: Vec<_> = wild::args().skip(1).collect();
    let has_some_explicit_args = !args.is_empty();
    let mut m = match opts.parse(args) {
        Ok(m) => m,
        Err(err) => {
            eprintln!("{}", err);
            process::exit(2);
        },
    };

    let posterize = m.opt_str("posterize").and_then(|p| p.parse().ok()).unwrap_or(0);
    let speed = m.opt_str("speed").and_then(|p| p.parse().ok()).unwrap_or(0);
    let floyd = m.opt_str("floyd").and_then(|p| p.parse().ok()).unwrap_or(1.);

    let quality = m.opt_str("quality").and_then(|s| CString::new(s).ok());
    let extension = m.opt_str("ext").and_then(|s| CString::new(s).ok());
    let map_file = m.opt_str("map").and_then(|s| CString::new(s).ok());

    let colors = if let Some(c) = m.free.get(0).and_then(|s| s.parse().ok()) {
        m.free.remove(0);
        if m.free.len() == 0 {
            m.free.push("-".to_owned()); // stdin default
        }
        c
    } else {0};
    let using_stdin = m.free.len() == 1 && Some("-") == m.free.get(0).map(|s| s.as_str());
    let mut using_stdout = using_stdin;
    let output_file_path = m.opt_str("o").and_then(|s| {
        if s == "-" {
            using_stdout = true;
            None
        } else {
            using_stdout = false;
            CString::new(s).ok()
        }
    });

    let files: Vec<_> = m.free.drain(..).filter_map(|s| CString::new(s).ok()).collect();
    let file_ptrs: Vec<_> = files.iter().map(|s| s.as_ptr()).collect();

    let verbose = m.opt_present("v");

    let print_version = m.opt_present("V");
    if print_version {
        println!("{} {}", env!("CARGO_PKG_VERSION"), VERSION_SUFFIX);
        return;
    }

    let print_help = m.opt_present("h");
    if print_help {
        let out = &mut std::io::stdout();
        print_full_version(out).unwrap();
        print_usage(out).unwrap();
        return;
    }

    let missing_arguments = !has_some_explicit_args;
    if missing_arguments {
        let out = &mut std::io::stderr();
        print_full_version(out).unwrap();
        print_usage(out).unwrap();
        process::exit(1);
    }

    if !using_stdin && files.is_empty() {
        let out = &mut std::io::stderr();
        out.write_all(b"error: No input files specified\n\n").unwrap();
        if verbose {
            print_full_version(out).unwrap();
        }
        print_usage(out).unwrap();
        process::exit(1);
    }

    let mut options = pngquant_options {
        quality: unwrap_ptr(quality.as_ref()),
        extension: unwrap_ptr(extension.as_ref()),
        output_file_path: unwrap_ptr(output_file_path.as_ref()),
        map_file: unwrap_ptr(map_file.as_ref()),
        files: file_ptrs.as_ptr(),
        num_files: file_ptrs.len() as c_uint,
        using_stdin,
        using_stdout,
        missing_arguments,
        colors,
        speed,
        posterize,
        floyd,
        force: m.opt_present("force") && !m.opt_present("no-force"),
        skip_if_larger: m.opt_present("skip-if-larger"),
        strip: m.opt_present("strip"),
        iebug: m.opt_present("iebug"),
        last_index_transparent: m.opt_present("transbug"),
        print_help,
        print_version,
        verbose,

        liq: ptr::null_mut(),
        fixed_palette_image: ptr::null_mut(),
        log_callback: None,
        log_callback_user_info: ptr::null_mut(),
        fast_compression: false,
        min_quality_limit: false,
    };

    if m.opt_present("nofs") || m.opt_present("ordered") {
        options.floyd = 0.;
    }

    process::exit(unsafe {pngquant_main(&mut options)});
}
