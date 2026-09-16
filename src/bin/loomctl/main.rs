use loom_dsp::Mode;
use loom_ipc::{IpcClient, Request, Response};

fn print_usage() {
    println!("loomctl <command> <value>");
    println!("Commands:");
    println!("volume <float>\n Sets the volume");
    println!("mode <mode>\n Mode can be a string or a number. Valid Modes are:");
    println!("  0 Off");
    println!("  1 SpatialFilter");
    println!("  2 Surround3d");
    println!("  3 Ambience");
    println!("  4 Fidelity");
    println!("  5 Night");
    println!("  6 SpatialStereo");
    println!("  7 SpatialSurround");
    println!("pitch_enabled <bool>");
    println!(" Enables or disables pitch shifting");
    println!("pitch <float>");
    println!(" Sets the pitch");
}

fn handle_response(response: std::io::Result<Response>) {
    match response {
        Ok(Response::Ok) => {
            println!("Ok");
        }
        Ok(Response::Error) => {
            eprintln!("loomd: request failed");
        }
        Ok(Response::State {
            volume,
            mode,
            pitch_enabled,
            pitch,
        }) => {
            println!("Volume:        {volume}");
            println!("Mode:          {mode}");
            println!("Pitch enabled: {pitch_enabled}");
            println!("Pitch:         {pitch}");
        }
        Err(error) => {
            eprintln!("Failed to communicate with loomd: {error}");
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);

    let socket = loom_ipc::socket_path().expect("Could not resolve the Loom runtime socket");
    let mut ipc_client = IpcClient::new(socket).expect("Failed to connect to loomd");

    if let Some(command) = args.next() {
        match command.as_str() {
            "volume" => {
                if let Some(volume) = args.next() {
                    let volume: f32 = volume.parse().expect("Please pass a valid float");
                    handle_response(ipc_client.send(Request::SetVolume(volume)));
                } else {
                    eprintln!("Please provide a float for volume");
                }
            }
            "mode" => {
                if let Some(mode) = args.next() {
                    let mode = if let Ok(mode) = mode.parse() {
                        Mode::from_u8(mode)
                    } else if let Some(mode) = Mode::from_str(&mode) {
                        mode
                    } else {
                        eprintln!("Please provide a valid mode");
                        return;
                    };
                    handle_response(ipc_client.send(Request::SetMode(mode as u8)));
                } else {
                    eprintln!("Please provide a mode as string or number");
                    print_usage();
                }
            }
            "pitch_enabled" => {
                if let Some(pitch_enabled) = args.next() {
                    let pitch_enabled = pitch_enabled.parse().expect("Please pass a valid boolean");
                    handle_response(ipc_client.send(Request::SetPitchEnabled(pitch_enabled)));
                } else {
                    eprintln!("Please provide a boolean for pitch_enabled");
                }
            }
            "pitch" => {
                if let Some(pitch) = args.next() {
                    let pitch = pitch.parse().expect("Please provide a float for pitch");
                    handle_response(ipc_client.send(Request::SetPitch(pitch)));
                } else {
                    eprintln!("Please provide a float for pitch");
                }
            }
            "get_state" => {
                handle_response(ipc_client.send(Request::GetState));
            }
            "help" => {
                print_usage();
            }
            _ => {
                eprint!("Invalid command found: {}", command);
                print_usage();
            }
        }
    }
}
