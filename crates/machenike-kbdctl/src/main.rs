use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process;

const ACPI_CALL: &str = "/proc/acpi/call";
const STATE_DIR: &str = "/var/lib/machenike-kbdctl";
const STATE_FILE: &str = "/var/lib/machenike-kbdctl/state";

const ZONES: [u8; 3] = [0x03, 0x04, 0x05];

const PALETTE: [(&str, Rgb); 8] = [
    ("white", Rgb { r: 255, g: 255, b: 255 }),
    ("blue", Rgb { r: 0, g: 0, b: 255 }),
    ("red", Rgb { r: 255, g: 0, b: 0 }),
    ("green", Rgb { r: 0, g: 255, b: 0 }),
    ("cyan", Rgb { r: 0, g: 255, b: 255 }),
    ("magenta", Rgb { r: 255, g: 0, b: 255 }),
    ("yellow", Rgb { r: 255, g: 255, b: 0 }),
    ("orange", Rgb { r: 255, g: 120, b: 0 }),
];

#[derive(Debug, Clone, Copy)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Debug, Clone, Copy)]
struct State {
    color: Rgb,
    brightness: u8,
    last_brightness: u8,
    enabled: bool,
    palette_index: usize,
}

fn usage() -> ! {
    eprintln!(
        r#"Usage:
  machenike-kbdctl red|green|blue|white|off
  machenike-kbdctl rgb <r> <g> <b>
  machenike-kbdctl zone <1|2|3> <r> <g> <b>

  machenike-kbdctl next
  machenike-kbdctl toggle
  machenike-kbdctl brightness <0..100>
  machenike-kbdctl brightness-up
  machenike-kbdctl brightness-down
  machenike-kbdctl status

Examples:
  sudo machenike-kbdctl next
  sudo machenike-kbdctl toggle
  sudo machenike-kbdctl brightness-up
  sudo machenike-kbdctl brightness-down
"#
    );

    process::exit(1);
}

fn parse_u8(value: &str) -> u8 {
    value.parse::<u8>().unwrap_or_else(|_| {
        eprintln!("Invalid value: {value}. Expected 0..255");
        process::exit(1);
    })
}

fn parse_brightness(value: &str) -> u8 {
    let value = parse_u8(value);

    if value > 100 {
        eprintln!("Invalid brightness: {value}. Expected 0..100");
        process::exit(1);
    }

    value
}

fn default_state() -> State {
    State {
        color: Rgb { r: 255, g: 255, b: 255 },
        brightness: 100,
        last_brightness: 100,
        enabled: true,
        palette_index: 0,
    }
}

fn read_state() -> State {
    let Ok(content) = fs::read_to_string(STATE_FILE) else {
        return default_state();
    };

    let parts: Vec<&str> = content.split_whitespace().collect();

    // Legacy state format:
    // r g b brightness
    if parts.len() == 4 {
        let Ok(r) = parts[0].parse::<u8>() else { return default_state(); };
        let Ok(g) = parts[1].parse::<u8>() else { return default_state(); };
        let Ok(b) = parts[2].parse::<u8>() else { return default_state(); };
        let Ok(brightness) = parts[3].parse::<u8>() else { return default_state(); };

        let brightness = brightness.min(100);

        return State {
            color: Rgb { r, g, b },
            brightness,
            last_brightness: if brightness == 0 { 100 } else { brightness },
            enabled: brightness != 0,
            palette_index: 0,
        };
    }

    // New state format:
    // r g b brightness last_brightness enabled palette_index
    if parts.len() != 7 {
        return default_state();
    }

    let Ok(r) = parts[0].parse::<u8>() else { return default_state(); };
    let Ok(g) = parts[1].parse::<u8>() else { return default_state(); };
    let Ok(b) = parts[2].parse::<u8>() else { return default_state(); };
    let Ok(brightness) = parts[3].parse::<u8>() else { return default_state(); };
    let Ok(last_brightness) = parts[4].parse::<u8>() else { return default_state(); };
    let Ok(enabled_raw) = parts[5].parse::<u8>() else { return default_state(); };
    let Ok(palette_index) = parts[6].parse::<usize>() else { return default_state(); };

    State {
        color: Rgb { r, g, b },
        brightness: brightness.min(100),
        last_brightness: last_brightness.min(100).max(10),
        enabled: enabled_raw != 0,
        palette_index: palette_index % PALETTE.len(),
    }
}

fn save_state(state: State) {
    if !Path::new(STATE_DIR).exists() {
        if let Err(error) = fs::create_dir_all(STATE_DIR) {
            eprintln!("Failed to create state directory: {error}");
            process::exit(1);
        }
    }

    let content = format!(
        "{} {} {} {} {} {} {}\n",
        state.color.r,
        state.color.g,
        state.color.b,
        state.brightness,
        state.last_brightness,
        if state.enabled { 1 } else { 0 },
        state.palette_index,
    );

    if let Err(error) = fs::write(STATE_FILE, content) {
        eprintln!("Failed to save state: {error}");
        process::exit(1);
    }
}

fn scale_color(color: Rgb, brightness: u8) -> Rgb {
    let scale = |value: u8| -> u8 {
        ((value as u16 * brightness as u16) / 100) as u8
    };

    Rgb {
        r: scale(color.r),
        g: scale(color.g),
        b: scale(color.b),
    }
}

fn write_acpi_call(call: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).open(ACPI_CALL)?;
    writeln!(file, "{call}")?;
    Ok(())
}

fn set_zone_raw(zone: u8, color: Rgb) -> std::io::Result<()> {
    // Protocol:
    // \_SB.PC00.LPCB.EC.ECMD {0x05, 0x00, 0xCA, zone, B, R, G, 0x00}
    let call = format!(
        "\\_SB.PC00.LPCB.EC.ECMD {{0x05, 0x00, 0xCA, 0x{zone:02X}, 0x{b:02X}, 0x{r:02X}, 0x{g:02X}, 0x00}}",
        zone = zone,
        b = color.b,
        r = color.r,
        g = color.g,
    );

    write_acpi_call(&call)
}

fn apply_state(state: State) {
    let brightness = if state.enabled { state.brightness } else { 0 };
    let real_color = scale_color(state.color, brightness);

    for zone in ZONES {
        if let Err(error) = set_zone_raw(zone, real_color) {
            eprintln!("Failed to write ACPI call: {error}");
            eprintln!();
            eprintln!("Hints:");
            eprintln!("  sudo modprobe acpi_call");
            eprintln!("  sudo machenike-kbdctl white");
            process::exit(1);
        }
    }
}

fn set_color(color: Rgb, palette_index: Option<usize>) {
    let mut state = read_state();

    state.color = color;
    state.enabled = true;

    if state.brightness == 0 {
        state.brightness = state.last_brightness.max(10);
    }

    if let Some(index) = palette_index {
        state.palette_index = index % PALETTE.len();
    }

    save_state(state);
    apply_state(state);
}

fn set_brightness(brightness: u8) {
    let mut state = read_state();

    state.brightness = brightness;

    if brightness > 0 {
        state.last_brightness = brightness;
        state.enabled = true;
    } else {
        state.enabled = false;
    }

    save_state(state);
    apply_state(state);
}

fn change_brightness(delta: i16) {
    let mut state = read_state();

    let current = if state.enabled { state.brightness } else { state.last_brightness };
    let next = (current as i16 + delta).clamp(0, 100) as u8;

    state.brightness = next;

    if next > 0 {
        state.last_brightness = next;
        state.enabled = true;
    } else {
        state.enabled = false;
    }

    save_state(state);
    apply_state(state);
}

fn next_color() {
    let mut state = read_state();

    let next_index = (state.palette_index + 1) % PALETTE.len();
    let (_, color) = PALETTE[next_index];

    state.palette_index = next_index;
    state.color = color;
    state.enabled = true;

    if state.brightness == 0 {
        state.brightness = state.last_brightness.max(10);
    }

    save_state(state);
    apply_state(state);

    println!("color: {}", PALETTE[next_index].0);
}

fn toggle() {
    let mut state = read_state();

    if state.enabled {
        if state.brightness > 0 {
            state.last_brightness = state.brightness;
        }

        state.enabled = false;
    } else {
        state.enabled = true;

        if state.brightness == 0 {
            state.brightness = state.last_brightness.max(10);
        }
    }

    save_state(state);
    apply_state(state);

    println!("enabled: {}", state.enabled);
}

fn status() {
    let state = read_state();

    println!("enabled: {}", state.enabled);
    println!("color: {} {} {}", state.color.r, state.color.g, state.color.b);
    println!("brightness: {}%", state.brightness);
    println!("last_brightness: {}%", state.last_brightness);
    println!("palette: {} ({})", state.palette_index, PALETTE[state.palette_index].0);

    let brightness = if state.enabled { state.brightness } else { 0 };
    let real_color = scale_color(state.color, brightness);

    println!("applied: {} {} {}", real_color.r, real_color.g, real_color.b);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("red") => set_color(Rgb { r: 255, g: 0, b: 0 }, Some(2)),
        Some("green") => set_color(Rgb { r: 0, g: 255, b: 0 }, Some(3)),
        Some("blue") => set_color(Rgb { r: 0, g: 0, b: 255 }, Some(1)),
        Some("white") => set_color(Rgb { r: 255, g: 255, b: 255 }, Some(0)),
        Some("off") => set_brightness(0),

        Some("rgb") if args.len() == 5 => {
            let color = Rgb {
                r: parse_u8(&args[2]),
                g: parse_u8(&args[3]),
                b: parse_u8(&args[4]),
            };

            set_color(color, None);
        }

        Some("zone") if args.len() == 6 => {
            let zone_number = parse_u8(&args[2]);

            if !(1..=3).contains(&zone_number) {
                eprintln!("Invalid zone: {zone_number}. Expected 1, 2, or 3");
                process::exit(1);
            }

            let zone = ZONES[(zone_number - 1) as usize];

            let color = Rgb {
                r: parse_u8(&args[3]),
                g: parse_u8(&args[4]),
                b: parse_u8(&args[5]),
            };

            if let Err(error) = set_zone_raw(zone, color) {
                eprintln!("Failed to write ACPI call: {error}");
                process::exit(1);
            }
        }

        Some("next") => next_color(),
        Some("toggle") => toggle(),

        Some("brightness") if args.len() == 3 => {
            let brightness = parse_brightness(&args[2]);
            set_brightness(brightness);
        }

        Some("brightness-up") => change_brightness(10),
        Some("brightness-down") => change_brightness(-10),

        Some("status") => status(),

        _ => usage(),
    }
}
