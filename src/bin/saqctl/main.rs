use clap::{Parser, Subcommand};
use saq_dsp::{EqPreset, Mode};
use saq_ipc::{IpcClient, Request, Response};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "saqctl",
    about = "CLI for the Śaq daemon",
    arg_required_else_help = true,
    after_help = "Examples:\n  saqctl status\n  saqctl volume 0.75\n  saqctl mode spatial-surround\n  saqctl pitch enable\n  saqctl pitch set -2.5\n  saqctl eq dialogue"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Status,
    Volume {
        level: f32,
    },
    Mode {
        #[arg(value_enum)]
        mode: Mode,
    },
    Pitch {
        #[command(subcommand)]
        command: PitchCommand,
    },
    Subwoofer {
        position: f32,
    },
    Eq {
        #[arg(value_enum)]
        preset: EqPreset,
    },
}

#[derive(Subcommand)]
enum PitchCommand {
    Enable,
    Disable,
    Set {
        #[arg(allow_negative_numbers = true)]
        semitones: f32,
    },
}

impl From<Command> for Request {
    fn from(command: Command) -> Self {
        match command {
            Command::Status => Self::GetState,
            Command::Volume { level } => Self::SetVolume(level),
            Command::Mode { mode } => Self::SetMode(mode as u8),
            Command::Pitch { command } => match command {
                PitchCommand::Enable => Self::SetPitchEnabled(true),
                PitchCommand::Disable => Self::SetPitchEnabled(false),
                PitchCommand::Set { semitones } => Self::SetPitch(semitones),
            },
            Command::Subwoofer { position } => Self::SetSubwoofer(position),
            Command::Eq { preset } => Self::SetEqPreset(preset as u8),
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let response = (|| {
        let mut client = IpcClient::new(
            saq_ipc::socket_path()
                .map_err(|error| format!("cannot locate saqd runtime path: {error}"))?,
        )
        .map_err(|error| format!("cannot connect to saqd: {error}"))?;
        client
            .send(cli.command.into())
            .map_err(|error| format!("communication with saqd failed: {error}"))
    })();

    match response {
        Ok(Response::Ok) => {
            println!("ok");
            ExitCode::SUCCESS
        }
        Ok(Response::Error(error)) => {
            eprintln!("saqctl: daemon rejected the request: {error}");
            ExitCode::FAILURE
        }
        Ok(Response::State {
            volume,
            mode,
            pitch_enabled,
            pitch,
            subwoofer,
            eq_preset,
            ..
        }) => {
            println!("Volume:        {volume:.2}");
            println!("Mode:          {}", Mode::from_u8(mode));
            println!("Pitch:         {pitch:+.2} semitones");
            println!("Pitch enabled: {pitch_enabled}");
            println!("Subwoofer:     {subwoofer:.2}");
            println!("EQ:            {}", EqPreset::from_u8(eq_preset).label());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("saqctl: {error}");
            ExitCode::FAILURE
        }
    }
}
