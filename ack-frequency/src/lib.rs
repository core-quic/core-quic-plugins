use pluginop_wasm::{PluginEnv, PluginCell, quic::{ConnectionField, FrameRegistration, FrameSendOrder, FrameSendKind, QVal, PacketType, PacketNumberSpaceField, Frame, PathChallengeFrame, Registration}, UnixInstant, Bytes, Duration};
use lazy_static::lazy_static;

struct PluginData {
    min_ack: Option<Duration>,
    last_sent: Option<UnixInstant>,
}

lazy_static! {
    static ref PLUGIN_DATA: PluginCell<PluginData> = PluginCell::new(PluginData { 
        min_ack: None,
        last_sent: None,
    });
}

// Initialize the plugin.
#[no_mangle]
pub extern fn init(penv: &mut PluginEnv) -> i64 {
    if penv.register(Registration::TransportParameter(0xff04de1b)).is_err() {
        return -2;
    }
    match penv.register(Registration::Frame(FrameRegistration::new(0xaf, FrameSendOrder::AfterACK, FrameSendKind::OncePerPacket, true, true))) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern fn decode_transport_parameter_ff04de1b(penv: &mut PluginEnv) -> i64 {
    let bytes = match penv.get_input::<Bytes>(0) {
        Ok(b) => b,
        _ => return -1,
    };
    // TODO: varint encoding.
    // We know the length to read.
    let min_af = match penv.get_bytes(bytes.tag, bytes.max_read_len) {
        Ok(v) => v,
        _ => return -2,
    };
    let mut new = vec![0; 8 - min_af.len()];
    new.extend(min_af);
    penv.print(&format!("AF got val {:x?}", new));
    PLUGIN_DATA.get_mut().min_ack = Some(Duration::from_micros(u64::from_be_bytes(new.try_into().unwrap())));
    penv.print(&format!("AF WORKS! Got value {:?}", PLUGIN_DATA.min_ack));
    0
}

#[no_mangle]
pub extern fn write_transport_parameter_ff04de1b(penv: &mut PluginEnv) -> i64 {
    penv.print("CALLED TO WRITE!");
    let bytes = match penv.get_input::<Bytes>(0) {
        Ok(b) => b,
        _ => return -1,
    };
    // TODO: check if there is at least 3 bytes.
    // TODO: let's force 10 ms.
    let tp_bytes: [u8; 11] = [0xc0, 0x00, 0x00, 0x00, 0xff, 0x04, 0xde, 0x1b, 0x02, 0x27, 0x10];
    match penv.put_bytes(bytes.tag, &tp_bytes) {
        Ok(11) => {},
        _ => return -4,
    };
    penv.print("WRITE OK");
    0
}

#[no_mangle]
pub extern fn should_send_frame_2(penv: &mut PluginEnv) -> i64 {
    // Such a behavior could be improved by avoiding loading this operation
    // if the negotiation failed.
    penv.print("CHECK should send ack!");
    // Always assume path is active.
    let now = match penv.get_input::<UnixInstant>(4) {
        Ok(n) => n,
        _ => return -7,
    };
    let pkt_type = match penv.get_input::<QVal>(0) {
        Ok(QVal::PacketType(pt)) => pt,
        _ => return -1,
    };
    let epoch = match penv.get_input::<QVal>(1) {
        Ok(QVal::PacketNumberSpace(p)) => p,
        _ => return -9,
    };
    let ack_elicited = match penv.get_connection::<bool>(ConnectionField::PacketNumberSpace(epoch, PacketNumberSpaceField::AckEllicited)) {
        Ok(b) => b,
        _ => return -4,
    };
    let need_ack_len = match penv.get_connection::<bool>(ConnectionField::PacketNumberSpace(epoch, PacketNumberSpaceField::ReceivedPacketNeedAck)) {
        Ok(b) => b,
        _ => return -3,
    };
    let is_closing = match penv.get_input::<bool>(2) {
        Ok(b) => b,
        _ => return -2,
    };
    penv.print("Going to evaluate");
    let out = if let (Some(min_af), Some(last)) = (PLUGIN_DATA.min_ack.as_ref(), PLUGIN_DATA.last_sent.as_ref()) {
        penv.print("Entering if");
        if *last + *min_af <= now {
            need_ack_len &&
            ack_elicited &&
            (!is_closing ||
                pkt_type == PacketType::Handshake)
        } else {
            penv.print("FALSE");
            false
        }
    } else {
       need_ack_len &&
            ack_elicited &&
            (!is_closing ||
                pkt_type == PacketType::Handshake)
    };
    penv.print("YAY");
    if out {
        penv.print("TRUE");
        PLUGIN_DATA.get_mut().last_sent = Some(now);
    }
    match penv.save_output(out.into()) {
        Ok(()) => 0,
        Err(_) => -6,
    }
}

