#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let msg = rtsp_types::Message::<&[u8]>::parse(data);

    if let Ok(m) = msg {
        let mut data = Vec::new();
        match m.0 {
            rtsp_types::Message::Request(req) => {
                let _ = req.write(&mut data);
            }
            rtsp_types::Message::Response(resp) => {
                let _ = resp.write(&mut data);
            }
            rtsp_types::Message::Data(d) => {
                let _ = d.write(&mut data);
            }
        }
    };
});
