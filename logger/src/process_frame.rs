use pluginop_wasm::{PluginEnv, PluginCell, UnixInstant, Bytes, quic::{QVal, Registration, Frame, ExtensionFrame, FrameSendKind, FrameSendOrder, FrameRegistration, PacketType}, fd::{FileDescriptor}};

use crate::record;

#[no_mangle]
pub extern fn post_process_frame_2(penv: &PluginEnv) -> i64 {
    penv.print("Called!");
    let ack_frame = match penv.get_input(0) {
        Ok(QVal::Frame(Frame::ACK(af))) => af,
        _ => unreachable!(),
    };
    record(penv, &format!("Processed ACK frame with largest ack {}, ack delay {} and nb of gaps {}",
                   ack_frame.largest_acknowledged, ack_frame.ack_delay, ack_frame.ack_range_count));
    0
}
