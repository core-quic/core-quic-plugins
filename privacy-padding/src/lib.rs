use pluginop_wasm::{PluginEnv, PluginCell, UnixInstant, Duration, quic::{QVal, ConnectionField, Registration, Frame, PaddingFrame, FrameSendKind, FrameSendOrder, FrameRegistration, PacketType}, Bytes};
use lazy_static::lazy_static;

#[derive(Debug)]
struct PluginData {
    stop_sending: bool,
    rng: Option<fastrand::Rng>,
    enable: bool,
}

lazy_static! {
    static ref PLUGIN_DATA: PluginCell<PluginData> = PluginCell::new(PluginData {
        stop_sending: false,
        rng: None,
        enable: true,
    });
}

// Initialize the plugin.
#[no_mangle]
pub extern fn init(penv: &mut PluginEnv) -> i64 {
    let now = match penv.get_unix_instant() {
        Ok(n) => n.secs() + n.subsec_nanos() as u64,
        Err(_) => return -1,
    };
    PLUGIN_DATA.get_mut().rng = Some(fastrand::Rng::with_seed(now));
    penv.enable();
    // Trick here.
    match penv.register(Registration::Frame(FrameRegistration::new(0xaaaa, FrameSendOrder::First, FrameSendKind::OncePerPacket, false, true))) {
        Ok(()) => {},
        _ => return -1,
    };
    match penv.register(Registration::Frame(FrameRegistration::new(0x0, FrameSendOrder::End, FrameSendKind::OncePerPacket, false, true))) {
        Ok(()) => 0,
        _ => -1,
    }
}

// This function determines if there are plugin frames that must be
// sent now or not.
#[no_mangle]
pub extern fn should_send_frame_0(penv: &mut PluginEnv) -> i64 {
    let pkt_type = match penv.get_input::<QVal>(0) {
        Ok(QVal::PacketType(pt)) => pt,
        _ => return -1,
    };
    let left = match penv.get_input::<usize>(3) {
        Ok(u) => u,
        _ => return -2,
    };
    let now = match penv.get_input::<UnixInstant>(4) {
        Ok(u) => u,
        _ => return -3,
    };
    let established: bool = match penv.get_connection(ConnectionField::IsEstablished) {
        Ok(b) => b,
        _ => return -5,
    };
    // Let suspend the sending if we are too early. This is done by calling prepare frame and giving an error.
    let out = left > 0 && (pkt_type != PacketType::Short || PLUGIN_DATA.enable);
    if pkt_type == PacketType::Short && established {
        PLUGIN_DATA.get_mut().stop_sending = true;
        let pause = PLUGIN_DATA.get_mut().rng.as_mut().unwrap().u64(..10000);
        let next_sending = now + Duration::from_micros(pause);
        if penv.set_timer(next_sending, 1, 7).is_err() {
            return -6;
        }
    }
    match penv.save_output(out.into()) {
        Ok(()) => 0,
        Err(_) => -4,
    }
}

#[no_mangle]
pub extern fn should_send_frame_aaaa(penv: &mut PluginEnv) -> i64 {
    match penv.save_output((PLUGIN_DATA.enable && PLUGIN_DATA.stop_sending).into()) {
        Ok(()) => 0,
        Err(_) => -4,
    }
}

#[no_mangle]
pub extern fn prepare_frame_0(penv: &mut PluginEnv) -> i64 {
    let left = match penv.get_input::<usize>(1) {
        Ok(u) => u,
        _ => return -2,
    };
    match penv.save_output(Frame::Padding(PaddingFrame { length: left as u64 }).into()) {
        Ok(()) => 0,
        _ => -1,
    }
}

#[no_mangle]
pub extern fn wire_len_0(penv: &mut PluginEnv) -> i64 {
    let p = match penv.get_input::<QVal>(0) {
        Ok(QVal::Frame(Frame::Padding(p))) => p,
        _ => return -2,
    };
    match penv.save_output((p.length as usize).into()) {
        Ok(()) => 0,
        _ => -1,
    }
}

#[no_mangle]
pub extern fn write_frame_0(penv: &mut PluginEnv) -> i64 {
    let p = match penv.get_input::<QVal>(0) {
        Ok(QVal::Frame(Frame::Padding(p))) => p,
        _ => return -2,
    };
    let bytes = match penv.get_input::<Bytes>(1) {
        Ok(b) => b,
        _ => return -3,
    };
    // TODO: check if there is at least 3 bytes.
    let frame_bytes: Vec<u8> = vec![0x00; p.length as usize];
    match penv.put_bytes(bytes.tag, &frame_bytes) {
        Ok(v) if v == p.length as usize => {},
        _ => return -4,
    };
    match penv.save_output(frame_bytes.len().into()) {
        Ok(()) => 0,
        _ => -1,
    }
}


#[no_mangle]
pub extern fn prepare_frame_aaaa(_penv: &mut PluginEnv) -> i64 {
    // Specific error code to stop the sending processing.
    return -1000;
}

#[no_mangle]
pub extern fn on_plugin_timeout_7(_penv: &mut PluginEnv) -> i64 {
    PLUGIN_DATA.get_mut().stop_sending = false;
    0
}

#[no_mangle]
pub extern fn plugin_control_80001(penv: &mut PluginEnv) -> i64 {
    let enable = match penv.get_input::<bool>(0) {
        Ok(n) => n,
        _ => return -2,
    };
    PLUGIN_DATA.get_mut().enable = enable;
    0
}