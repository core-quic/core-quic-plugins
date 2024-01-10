use std::format;

use pluginop_wasm::{PluginEnv, PluginCell, quic::ConnectionField, fd::FileDescriptor};
use lazy_static::lazy_static;

use std::io::Write;

struct PluginData {
    logging_fd: Option<FileDescriptor>,
}

lazy_static! {
    static ref PLUGIN_DATA: PluginCell<PluginData> = PluginCell::new(PluginData {logging_fd: None});
}

// Initialize the plugin.
#[no_mangle]
pub extern fn init(penv: &mut PluginEnv) -> i64 {
    penv.print("Initializing logger plugin");
    penv.enable();
    let role = if let Ok(v) = penv.get_connection(ConnectionField::IsServer) {
        match v {
            true => "server",
            false => "client",
        }
    } else {
        penv.print("aie 1");
        return -1;
    };
    let start_time = match penv.get_unix_instant() {
        Ok(ui) => ui,
        Err(_) => {
            penv.print("aie 2");
            return -2;
        },
    };
    match FileDescriptor::create(&format!("log-{}-{}{}.log", role, start_time.secs(), start_time.subsec_nanos())) {
        Ok(fd) => {
            PLUGIN_DATA.get_mut().logging_fd = Some(fd);
            penv.print("All good");
            0
        }
        Err(_) => {
            penv.print("aie 3");
            -3
        },
    }
}

fn record(penv: &PluginEnv, s: &str) {
    match PLUGIN_DATA.get_mut().logging_fd.as_mut() {
        Some(fd) => {
            let time = match penv.get_unix_instant() {
                Ok(t) => t,
                Err(_) => return,
            };
            match fd.write(&format!("{:?}: {}\n", time, s).as_bytes()) {
                Ok(w) => penv.print(&format!("written {} bytes", w)),
                Err(_) => penv.print("write failed"),
            }
        },
        None => penv.print("FD does not exist!"),
    }
}

pub mod process_frame;