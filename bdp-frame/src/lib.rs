use std::format;

use std::collections::HashMap;
use pluginop_wasm::{Duration, PluginEnv, PluginCell, Bytes, quic::{QVal, Registration, Frame, ExtensionFrame, FrameSendKind, FrameSendOrder, FrameRegistration, PacketType, RecoveryField}};
use lazy_static::lazy_static;

#[derive(Debug)]
struct FrameData {
    val: u8,
}

#[derive(Debug)]
struct PluginData {
    should_send: bool,
    previous_cwin: u64,
    cwin_to_set: u64,
}

const BD_FRAME_TYPE: u64 = 0xBD;

lazy_static! {
    static ref PLUGIN_DATA: PluginCell<PluginData> = PluginCell::new(PluginData {
        should_send: false,
        previous_cwin: 2000000,
        cwin_to_set: 0,
    });
}

// Initialize the plugin.
#[no_mangle]
pub extern fn init(penv: &mut PluginEnv) -> i64 {
    match penv.register(Registration::TransportParameter(0xBF)) {
        Ok(()) => (),
        _ => return -2,
    };
    match penv.register(Registration::Frame(FrameRegistration::new(BD_FRAME_TYPE, FrameSendOrder::First, FrameSendKind::OncePerPacket, true, true))) {
        Ok(()) => 0,
        _ => -1,
    }
}

#[no_mangle]
pub extern fn decode_transport_parameter_bf(penv: &mut PluginEnv) -> i64 {
    // This is a zero-length TP. We just got it.
    if let Ok(false) = penv.get_connection(pluginop_wasm::quic::ConnectionField::IsServer) {
        PLUGIN_DATA.get_mut().should_send = true;
    }
    penv.enable();
    0
}

#[no_mangle]
pub extern fn write_transport_parameter_bf(penv: &mut PluginEnv) -> i64 {
    let bytes = match penv.get_input::<Bytes>(0) {
        Ok(b) => b,
        _ => return -1,
    };
    // TODO: check if there is at least 3 bytes.
    // TODO: let's force 10 ms.
    let tp_bytes: [u8; 3] = [0x40, 0xbf, 0x00];
    match penv.put_bytes(bytes.tag, &tp_bytes) {
        Ok(3) => {},
        _ => return -4,
    };
    0
}

// This function determines if there are plugin frames that must be
// sent now or not.
#[no_mangle]
// pub extern fn should_send_frame_42(pkt_type: u32, _epoch: u64, is_closing: i32, _left: u64) -> i64 {
pub extern fn should_send_frame_bd(penv: &mut PluginEnv) -> i64 {
    let pkt_type = match penv.get_input::<QVal>(0) {
        Ok(QVal::PacketType(pt)) => pt,
        _ => return -1,
    };
    let is_closing = match penv.get_input::<bool>(2) {
        Ok(b) => b,
        _ => return -2,
    };
    let out = pkt_type == PacketType::Short && !is_closing && PLUGIN_DATA.should_send;
    match penv.save_output(out.into()) {
        Ok(()) => 0,
        Err(_) => -3,
    }
}

// This function is important, as it determines which (custom) frame
// should be sent. This is specified as the return value. This function
// is called if, and only if `should_send_frame` returns `true`.
//
// In case no frame should be sent, return u64::MAX.
//
// Note that when preparing this frame, a tag must be provided to the
// host implementation to retrieve the related data.
#[no_mangle]
pub extern fn prepare_frame_bd(penv: &mut PluginEnv) -> i64 {
    // We need to save the extension frame.
    match penv.save_output(Frame::Extension(ExtensionFrame { frame_type: BD_FRAME_TYPE, tag: 0 }).into()) {
        Ok(()) => 0,
        _ => -1,
    }
}

#[no_mangle]
pub extern fn write_frame_bd(penv: &mut PluginEnv) -> i64 {
    // No need to get the extension frame, we know what it should contain.
    let bytes = match penv.get_input::<Bytes>(1) {
        Ok(b) => b,
        _ => return -3,
    };
    // TODO: check if there is at least 3 bytes.
    let mut frame_bytes = [0u8; 10];
    frame_bytes[0..2].copy_from_slice(&[0x40, 0xbd]);
    frame_bytes[2..10].copy_from_slice(&PLUGIN_DATA.previous_cwin.to_be_bytes());
    match penv.put_bytes(bytes.tag, &frame_bytes) {
        Ok(10) => {},
        _ => return -4,
    };
    match penv.save_output(frame_bytes.len().into()) {
        Ok(()) => 0,
        _ => -5,
    }
}

#[no_mangle]
pub extern fn log_frame_bd(penv: &mut PluginEnv) -> i64 {
    // We know the content it should have.
    let bytes = match penv.get_input::<Bytes>(1) {
        Ok(b) => b,
        _ => return -2,
    };
    let s = format!("BDP_FRAME with cwin value {}", PLUGIN_DATA.previous_cwin);
    let s_bytes = s.into_bytes();
    let s_len = s_bytes.len();
    match penv.put_bytes(bytes.tag, &s_bytes) {
        Ok(l) if l == s_len => 0,
        _ => -3,
    }
}

#[no_mangle]
pub extern fn parse_frame_bd(penv: &mut PluginEnv) -> i64 {
    let bytes = match penv.get_input::<Bytes>(0) {
        Ok(b) => b,
        _ => return -1,
    };

    // Get the data, only one byte is actually needed to parse the val
    // (as the type frame is already parsed).
    let val = match penv.get_bytes(bytes.tag, 8) {
        Ok(v) => u64::from_be_bytes(v.try_into().expect("wut")),
        _ => return -2,
    };
    PLUGIN_DATA.get_mut().cwin_to_set = val;

    /* Don't forget this! */
    match penv.save_output(Frame::Extension(ExtensionFrame { frame_type: BD_FRAME_TYPE, tag: 0 }).into()) {
        Ok(()) => 0,
        _ => -3,
    }
}

#[no_mangle]
pub extern fn process_frame_bd(penv: &mut PluginEnv) -> i64 {
    // We know what to do here.
    if let Err(_) = penv.set_recovery(RecoveryField::CongestionWindow, PLUGIN_DATA.cwin_to_set as usize) {
        return -1;
    }
    if let Err(_) = penv.set_recovery(RecoveryField::Ssthresh, PLUGIN_DATA.cwin_to_set as usize) {
        return -2;
    }
    penv.print(&format!("Successfully set CWIN to {}", PLUGIN_DATA.cwin_to_set));
    0
}

#[no_mangle]
pub extern fn wire_len_bd(penv: &mut PluginEnv) -> i64 {
    // The frame always have the same size.
    let len: usize = 2 + 8; // Just the frame type and eight byte of data for now.
                            // And 0xbd needs 2 bytes...
    match penv.save_output(len.into()) {
        Ok(()) => 0,
        _ => -1,
    }
}

#[no_mangle]
pub extern fn on_frame_reserved_bd(penv: &mut PluginEnv) -> i64 {
    PLUGIN_DATA.get_mut().should_send = false;
    penv.print("BDP frame sent");
    0
}

#[no_mangle]
pub extern fn notify_frame_bd(penv: &mut PluginEnv) -> i64 {
    // is_lost is input 1
    let is_lost = match penv.get_input::<bool>(1) {
        Ok(l) => l,
        _ => return -1,
    };
    penv.print(&format!("BDP was lost? {}", is_lost));
    PLUGIN_DATA.get_mut().should_send = is_lost;
    0
}