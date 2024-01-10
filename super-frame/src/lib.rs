use std::format;

use std::collections::HashMap;
use pluginop_wasm::{PluginEnv, PluginCell, Bytes, quic::{QVal, Registration, Frame, ExtensionFrame, FrameSendKind, FrameSendOrder, FrameRegistration, PacketType}};
use lazy_static::lazy_static;

#[derive(Debug)]
struct FrameData {
    val: u8,
}

#[derive(Debug)]
struct PluginData {
    in_flight: bool,
    tag_count: u64,
    frames: HashMap<u64, FrameData>,
}

const SF_FRAME_TYPE: u64 = 0x42;

lazy_static! {
    static ref PLUGIN_DATA: PluginCell<PluginData> = PluginCell::new(PluginData {
        in_flight: false,
        tag_count: 0,
        frames: HashMap::new(),
    });
}

// Initialize the plugin.
#[no_mangle]
pub extern fn init(penv: &mut PluginEnv) -> i64 {
    match penv.register(Registration::TransportParameter(0xAAAAAAAA)) {
        Ok(()) => (),
        _ => return -2,
    };
    match penv.register(Registration::Frame(FrameRegistration::new(SF_FRAME_TYPE, FrameSendOrder::First, FrameSendKind::OncePerPacket, true, true))) {
        Ok(()) => 0,
        _ => -1,
    }
}

// Need to negotiate the SUPER frame...
// TODO TODO TODO
#[no_mangle]
pub extern fn decode_transport_parameter_aaaaaaaa(penv: &mut PluginEnv) -> i64 {
    // This is a zero-length TP. We just got it.
    penv.enable();
    0
}

#[no_mangle]
pub extern fn write_transport_parameter_aaaaaaaa(penv: &mut PluginEnv) -> i64 {
    let bytes = match penv.get_input::<Bytes>(0) {
        Ok(b) => b,
        _ => return -1,
    };
    // TODO: check if there is at least 3 bytes.
    // TODO: let's force 10 ms.
    let tp_bytes: [u8; 9] = [0xc0, 0x00, 0x00, 0x00, 0xaa, 0xaa, 0xaa, 0xaa, 0x00];
    match penv.put_bytes(bytes.tag, &tp_bytes) {
        Ok(9) => {},
        _ => return -4,
    };
    0
}

// This function determines if there are plugin frames that must be
// sent now or not.
#[no_mangle]
// pub extern fn should_send_frame_42(pkt_type: u32, _epoch: u64, is_closing: i32, _left: u64) -> i64 {
pub extern fn should_send_frame_42(penv: &mut PluginEnv) -> i64 {
    let pkt_type = match penv.get_input::<QVal>(0) {
        Ok(QVal::PacketType(pt)) => pt,
        _ => return -1,
    };
    let is_closing = match penv.get_input::<bool>(2) {
        Ok(b) => b,
        _ => return -2,
    };
    let out = pkt_type == PacketType::Short && !is_closing && !PLUGIN_DATA.in_flight;
    match penv.save_output(out.into()) {
        Ok(()) => 0,
        Err(_) => -3,
    }
}

// This is just a test to see if we can make PRE works.
// #[no_mangle]
// pub extern fn pre_should_send_frame_42(_pkt_type: u32, _epoch: u64, _is_closing: i32, _left: u64) {
    // print("Hello from pre_should_send_frame_custom");
// }

// This is just a test to see if we can make POST works.
// #[no_mangle]
// pub extern fn post_should_send_frame_42() {
    // print("Hello from post_should_send_frame_custom");
// }

// This function is important, as it determines which (custom) frame
// should be sent. This is specified as the return value. This function
// is called if, and only if `should_send_frame` returns `true`.
//
// In case no frame should be sent, return u64::MAX.
//
// Note that when preparing this frame, a tag must be provided to the
// host implementation to retrieve the related data.
#[no_mangle]
pub extern fn prepare_frame_42(penv: &mut PluginEnv) -> i64 {
    let tag = PLUGIN_DATA.tag_count;
    PLUGIN_DATA.get_mut().tag_count += 1;
    PLUGIN_DATA.get_mut().frames.insert(tag, FrameData { val: tag as u8});
    // We need to save the extension frame.
    match penv.save_output(Frame::Extension(ExtensionFrame { frame_type: SF_FRAME_TYPE, tag }).into()) {
        Ok(()) => 0,
        _ => -1,
    }
}

#[no_mangle]
pub extern fn write_frame_42(penv: &mut PluginEnv) -> i64 {
    let ext_frame = match penv.get_input::<QVal>(0) {
        Ok(QVal::Frame(Frame::Extension(e))) => e,
        _ => return -1,
    };
    let fd = match PLUGIN_DATA.frames.get(&ext_frame.tag) {
        Some(fd) => fd,
        _ => return -2,
    };
    let bytes = match penv.get_input::<Bytes>(1) {
        Ok(b) => b,
        _ => return -3,
    };
    // TODO: check if there is at least 3 bytes.
    let frame_bytes: [u8; 3] = [0x40, 0x42, fd.val];
    match penv.put_bytes(bytes.tag, &frame_bytes) {
        Ok(3) => {},
        _ => return -4,
    };
    match penv.save_output(frame_bytes.len().into()) {
        Ok(()) => 0,
        _ => -5,
    }
}

// Export a function named "log_frame_42".
#[no_mangle]
pub extern fn log_frame_42(penv: &mut PluginEnv) -> i64 {
    let ext_frame = match penv.get_input::<QVal>(0) {
        Ok(QVal::Frame(Frame::Extension(e))) => e,
        _ => return -1,
    };
    let bytes = match penv.get_input::<Bytes>(1) {
        Ok(b) => b,
        _ => return -2,
    };
    let s = match PLUGIN_DATA.frames.get(&ext_frame.tag) {
        Some(fd) => format!("my SUPER frame with type 0x42 and data {}", fd.val),
        None => "Invalid SUPER frame".to_string(),
    };
    let s_bytes = s.into_bytes();
    let s_len = s_bytes.len();
    match penv.put_bytes(bytes.tag, &s_bytes) {
        Ok(l) if l == s_len => 0,
        _ => -3,
    }
}

// Export a function named "parse_frame_42". This can then be called
// from the plugin crate!
#[no_mangle]
pub extern fn parse_frame_42(penv: &mut PluginEnv) -> i64 {
    let bytes = match penv.get_input::<Bytes>(0) {
        Ok(b) => b,
        _ => return -1,
    };
    /* Let have tag count */
    let tag = PLUGIN_DATA.tag_count;
    PLUGIN_DATA.get_mut().tag_count += 1;

    // Get the data, only one byte is actually needed to parse the val
    // (as the type frame is already parsed).
    let val = match penv.get_bytes(bytes.tag, 1) {
        Ok(v) => v,
        _ => return -2,
    };
    PLUGIN_DATA.get_mut().frames.insert(tag, FrameData { val: val[0] });

    /* Don't forget this! */
    match penv.save_output(Frame::Extension(ExtensionFrame { frame_type: SF_FRAME_TYPE, tag }).into()) {
        Ok(()) => 0,
        _ => -3,
    }
}

#[no_mangle]
pub extern fn process_frame_42(penv: &mut PluginEnv) -> i64 {
    /* Retrieve my data */
    // let fd = get_frame_data(tag);
    // No processing, no error
    penv.print("Successfully processed SUPER frame");
    0
}

#[no_mangle]
pub extern fn wire_len_42(penv: &mut PluginEnv) -> i64 {
    // Note that we might need the tag to infer the size.
    let len: usize = 2 + 1; // Just the frame type and one byte of data for now.
                            // And 0x42 needs 2 bytes...
    match penv.save_output(len.into()) {
        Ok(()) => 0,
        _ => -1,
    }
}

#[no_mangle]
pub extern fn on_frame_reserved_42(penv: &mut PluginEnv) -> i64 {
    PLUGIN_DATA.get_mut().in_flight = true;
    penv.print("SUPER frame sent");
    0
}

#[no_mangle]
pub extern fn notify_frame_42(penv: &mut PluginEnv) -> i64 {
    penv.print("Getting notification for SUPER frame");
    let ext_frame = match penv.get_input::<QVal>(0) {
        Ok(QVal::Frame(Frame::Extension(e))) => e,
        _ => return -1,
    };
    // is_lost is input 1
    PLUGIN_DATA.get_mut().frames.remove(&ext_frame.tag);
    PLUGIN_DATA.get_mut().in_flight = false;
    0
}