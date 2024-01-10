use pluginop_wasm::{Bytes, PluginEnv, PluginCell, quic::{FrameRegistration, FrameSendOrder, FrameSendKind, QVal, PacketType, Frame, PathChallengeFrame}, UnixInstant};
use lazy_static::lazy_static;

struct PluginData {
    need_challenge: bool,
    challenge_time: Vec<(u64, UnixInstant)>,
}

lazy_static! {
    static ref PLUGIN_DATA: PluginCell<PluginData> = PluginCell::new(PluginData {need_challenge: false, challenge_time: vec![]});
}

// Initialize the plugin.
#[no_mangle]
pub extern fn init(penv: &mut PluginEnv) -> i64 {
    penv.enable();
    match penv.register(pluginop_wasm::quic::Registration::Frame(FrameRegistration::new(0x1a, FrameSendOrder::AfterACK, FrameSendKind::OncePerPacket, false, true))) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern fn should_send_frame_1a(penv: &mut PluginEnv) -> i64 {
    let pkt_type = match penv.get_input::<QVal>(0) {
        Ok(QVal::PacketType(pt)) => pt,
        _ => return -1,
    };
    let is_closing = match penv.get_input::<bool>(2) {
        Ok(b) => b,
        _ => return -2,
    };
    let out = pkt_type == PacketType::Short && !is_closing && PLUGIN_DATA.need_challenge;
    penv.print(&format!("CALLED: out is {out}"));
    match penv.save_output(out.into()) {
        Ok(()) => 0,
        Err(_) => -3,
    }
}

#[no_mangle]
pub extern fn prepare_frame_1a(penv: &mut PluginEnv) -> i64 {
    let now = match penv.get_unix_instant() {
        Ok(n) => n.secs() + n.subsec_nanos() as u64,
        Err(_) => return -1,
    };
    let mut base1 = fastrand::Rng::with_seed(now);
    let data = base1.u64(..);
    penv.print(&format!("Got data {data:x}"));
    match penv.save_output(QVal::Frame(Frame::PathChallenge(PathChallengeFrame {
        data
    })).into()) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern fn write_frame_1a(penv: &mut PluginEnv) -> i64 {
    let pc_frame = match penv.get_input::<QVal>(0) {
        Ok(QVal::Frame(Frame::PathChallenge(pc))) => pc,
        _ => return -1,
    };
    let bytes = match penv.get_input::<Bytes>(1) {
        Ok(b) => b,
        _ => return -3,
    };
    // TODO: check if there is at least 3 bytes.
    let mut frame_bytes: Vec<u8> = vec![0x1a];
    frame_bytes.extend_from_slice(&pc_frame.data.to_be_bytes());
    match penv.put_bytes(bytes.tag, &frame_bytes) {
        Ok(9) => {},
        _ => return -4,
    };
    match penv.save_output(frame_bytes.len().into()) {
        Ok(()) => 0,
        _ => -5,
    }
}

#[no_mangle]
pub extern fn on_frame_reserved_1a(penv: &mut PluginEnv) -> i64 {
    let pc = match penv.get_input::<QVal>(0) {
        Ok(QVal::Frame(Frame::PathChallenge(pc))) => pc,
        _ => return -1,
    };
    let now = match penv.get_unix_instant() {
        Ok(n) => n,
        _ => return -2,
    };
    PLUGIN_DATA.get_mut().challenge_time.push((pc.data, now));
    PLUGIN_DATA.get_mut().need_challenge = false;
    0
}

#[no_mangle]
pub extern fn pre_process_frame_1b(penv: &mut PluginEnv) -> i64 {
    let pr = match penv.get_input::<QVal>(0) {
        Ok(QVal::Frame(Frame::PathResponse(pr))) => pr,
        _ => return -1,
    };
    let now = match penv.get_unix_instant() {
        Ok(n) => n,
        _ => return -2,
    };
    PLUGIN_DATA.get_mut().challenge_time.retain(|(cd, ct)| {
        if *cd == pr.data {
            let diff = now - *ct;
            penv.print(&format!("PC-PR Duration {:?}", diff));
            false
        } else {
            true
        }
    });
    0
}

#[no_mangle]
pub extern fn plugin_control_1(penv: &mut PluginEnv) -> i64 {
    // We assume we want to record the next PATH_CHALLENGE to send.
    PLUGIN_DATA.get_mut().need_challenge = true;
    penv.print("Requesting challenge sending");
    0
}